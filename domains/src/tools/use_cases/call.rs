// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{Inference, models::Conversation},
    tools::{
        Tools,
        models::{Call, Tool, ToolPermission, ToolsConfiguration},
        traits::ToolsProvider,
        use_cases::subagent,
    },
};

async fn add_tool_message_to_conversation(
    _ctx: &AppContext,
    conversation: &mut Conversation,
    call: &Call,
    response_type: &str,
    response: &str,
) -> Result<(), AppError> {
    conversation.add_tool_message(format!(
        "Name: {}\nArguments: {}\n{}: \n{}",
        call.name,
        call.arguments_as_json(),
        response_type,
        response
    ));

    Ok(())
}

fn get_permission_for_tool(
    ctx: &AppContext,
    tools_configuration: &ToolsConfiguration,
    tool_name: &str,
) -> Result<ToolPermission, AppError> {
    tools_configuration
        .get_permission_for_tool(tool_name)
        .ok_or_else(|| {
            app_error!(
                Unauthorized,
                "tool_permission_denied",
                &format!("Agent does not have permission to call tool: {}", tool_name),
                ctx.clone()
            )
        })
}

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

    /// Runs a provider tool outside of inference, e.g. for flow steps. The caller
    /// grants the permissions and records the result in the conversation.
    pub async fn execute_direct(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        permissions: ToolPermission,
        call: &Call,
    ) -> Result<String, AppError> {
        let Some((provider, tool)) = self.find_provider(ctx, &call.name).await else {
            return Err(app_error!(
                NotFound,
                "tool_not_found",
                &format!("Tool with name '{}' not found", call.name),
                ctx.clone()
            ));
        };

        if !call.validate_arguments(tool.schema) {
            return Err(app_error!(
                Validation,
                "tool_arguments_invalid",
                &format!(
                    "Arguments for tool '{}' are invalid please check schema",
                    call.name
                ),
                ctx.clone()
            ));
        }

        provider.execute(ctx, conversation, permissions, call).await
    }

    pub async fn call(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        inference: &Inference,
        call: &Call,
    ) -> Result<(), AppError> {
        ctx.logger
            .trace(
                ctx,
                &format!(
                    "Agent '{}' is calling tool '{}'",
                    conversation.agent.metadata.name, call.name
                ),
            )
            .await;

        let permissions = match get_permission_for_tool(ctx, &conversation.agent.tools, &call.name)
        {
            Ok(p) => p,
            Err(err) => {
                ctx.logger
                    .warn(
                        ctx,
                        &format!(
                            "Agent '{}' has no permission to access tool '{}'.",
                            conversation.agent.metadata.name, call.name
                        ),
                    )
                    .await;
                add_tool_message_to_conversation(
                    ctx,
                    conversation,
                    call,
                    "Error",
                    &err.internal_message,
                )
                .await?;
                return Ok(());
            }
        };

        if let Some((provider, tool)) = self.find_provider(ctx, &call.name).await {
            if !call.validate_arguments(tool.schema) {
                add_tool_message_to_conversation(
                    ctx,
                    conversation,
                    call,
                    "Error",
                    "Arguments for tool are invalid please check schema",
                )
                .await?;
                return Ok(());
            }

            match provider.execute(ctx, conversation, permissions, call).await {
                Ok(res) => {
                    add_tool_message_to_conversation(ctx, conversation, call, "Result", &res)
                        .await?
                }
                Err(err) => {
                    ctx.logger
                        .warn(
                            ctx,
                            &format!("Tool '{}' failed with: {}", call.name, err.internal_message),
                        )
                        .await;
                    add_tool_message_to_conversation(
                        ctx,
                        conversation,
                        call,
                        "Error",
                        &err.internal_message,
                    )
                    .await?
                }
            };

            return Ok(());
        }

        if call.name == subagent::LIST || call.name == subagent::INVOKE {
            if let Err(err) = self
                .call_subagent_tool(ctx, conversation, permissions, call, inference)
                .await
            {
                ctx.logger
                    .warn(
                        ctx,
                        &format!("Tool '{}' failed with: {}", call.name, err.internal_message),
                    )
                    .await;
                add_tool_message_to_conversation(
                    ctx,
                    conversation,
                    call,
                    "Error",
                    &err.internal_message,
                )
                .await?
            }

            return Ok(());
        }

        ctx.logger
            .warn(ctx, &format!("Tool '{}' not found", call.name))
            .await;
        add_tool_message_to_conversation(
            ctx,
            conversation,
            call,
            "Error",
            &format!("Tool with name '{}' not found", call.name),
        )
        .await
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
}
