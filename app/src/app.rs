// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{collections::HashMap, sync::Arc};

use app_domains::{
    core::{
        models::{AppContext, AppError},
        traits::ContextFactory,
    },
    inference::models::Conversation,
};

use crate::Domains;

pub struct App {
    pub domains: Domains,
    boot_context: AppContext,
    context_factory: Arc<dyn ContextFactory + Send + Sync>,
}

impl App {
    pub async fn new(
        boot_context: AppContext,
        context_factory: Arc<dyn ContextFactory + Send + Sync>,
    ) -> Result<Self, AppError> {
        Ok(Self {
            domains: Domains::new(&boot_context).await?,
            boot_context,
            context_factory,
        })
    }

    /// The context the application was bootstrapped with, for use outside of a request.
    pub fn boot_context(&self) -> &AppContext {
        &self.boot_context
    }

    pub async fn new_conversation(
        &self,
        ctx: &AppContext,
        agent_name: &str,
    ) -> Result<Conversation, AppError> {
        let chroot = ctx
            .configuration()
            .get_string(ctx, "context.chroot")
            .await?;
        self.domains
            .inference
            .new_conversation(ctx, agent_name, &self.domains.tools, &chroot)
            .await
    }

    pub async fn infer(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
    ) -> Result<(), AppError> {
        self.domains
            .inference
            .infer(ctx, conversation, self.domains.tools.as_ref())
            .await
    }

    pub async fn get_context(
        &self,
        trace_id: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<AppContext, AppError> {
        self.context_factory
            .create(&self.boot_context, trace_id, metadata)
            .await
    }
}
