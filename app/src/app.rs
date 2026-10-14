// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::configuration::ConfigurationUniversal;
use app_domains::{
    core::{
        models::{AppContext, AppError},
        traits::Logger,
    },
    inference::models::Conversation,
};

use crate::Domains;

pub struct App {
    pub domains: Domains,
    pub logger: Arc<dyn Logger + Send + Sync>,
}

impl App {
    pub async fn new(
        ctx: &AppContext,
        logger: Arc<dyn Logger + Send + Sync>,
    ) -> Result<Self, AppError> {
        Ok(Self {
            domains: Domains::new(ctx).await?,
            logger,
        })
    }

    pub async fn new_conversation(
        &self,
        ctx: &AppContext,
        agent_name: &str,
    ) -> Result<Conversation, AppError> {
        let chroot = ctx.configuration.get_string(ctx, "context.chroot").await?;
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

    pub async fn get_context(&self, id: String) -> Result<AppContext, AppError> {
        let configuration = Arc::new(ConfigurationUniversal::new()?);

        Ok(AppContext::new(id, configuration, self.logger.clone()))
    }
}
