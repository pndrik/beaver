// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;
use tracing::info;

use super::redirect::REDIRECT_ARGUMENT;
use crate::{
    core::models::{AppContext, AppError},
    inference::{Inference, models::Conversation},
    tools::{
        Tools,
        models::{Call, Tool, ToolPermission},
        traits::ToolsProvider,
        use_cases::subagent,
    },
};

mod helper;
use helper::*;

impl Tools {
    async fn find_provider(
        &self,
        ctx: &AppContext,
        tool_name: &str,
    ) -> Option<(Arc<dyn ToolsProvider + Send + Sync>, Tool)> {
        for provider in &self.tools_providers {
            if let Ok(tool) = provider.get(ctx, tool_name).await {
                return Some((provider.clone(), tool));
            }
        }

        None
    }

    pub async fn call_many(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        inference: &Inference,
        calls: Vec<Call>,
    ) -> Result<(), AppError> {
        for call in calls {
            self.call(ctx, conversation, inference, &call).await?;
        }

        Ok(())
    }

    pub async fn call(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        inference: &Inference,
        call: &Call,
    ) -> Result<(), AppError> {
        let outcome = async {
            let permissions = permission(ctx, conversation.agent(), call)?;

            if call.name == subagent::LIST || call.name == subagent::INVOKE {
                self.call_subagent_tool(ctx, conversation, permissions, call, inference)
                    .await?;
                return Ok(None);
            }

            let redirect_to = get_redirect(call);
            if let Some(path) = &redirect_to {
                redirect_allowed(ctx, conversation.agent(), call)?;
                conversation.chroot().resolve(ctx, path).await?;
            }

            let output = self
                .call_direct(ctx, conversation, permissions, call)
                .await?;

            match redirect_to {
                Some(path) => write_to_file(ctx, call, conversation.chroot(), &path, &output)
                    .await
                    .map(Some),
                None => Ok(Some(result_message(call, &output))),
            }
        }
        .await;

        match outcome {
            Ok(Some(message)) => append_message(conversation, message),
            Ok(None) => {}
            Err(err) => append_error(conversation, call, &err),
        }

        Ok(())
    }

    pub async fn call_direct(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        permissions: ToolPermission,
        call: &Call,
    ) -> Result<String, AppError> {
        info!(
            channel = "audit",
            object = "tool",
            action = "call",
            agent = conversation.agent().metadata.name,
            tool = call.name
        );

        let (provider, tool) = self
            .find_provider(ctx, &call.name)
            .await
            .ok_or_else(|| not_found(ctx, call))?;

        let mut call = call.clone();
        call.arguments.remove(REDIRECT_ARGUMENT);
        validate_arguments(ctx, &tool, &call)?;

        provider
            .execute(ctx, conversation, permissions, &call)
            .await
    }
}
