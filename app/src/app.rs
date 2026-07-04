// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_domains::core::models::{AppContext, AppError};

use crate::Domains;

pub struct App {
    pub domains: Domains,
}

impl App {
    pub async fn new(ctx: &AppContext) -> Result<Self, AppError> {
        Ok(Self {
            domains: Domains::new(ctx).await?,
        })
    }

    pub async fn new_conversation(
        &self,
        ctx: &AppContext,
        agent_name: &str,
    ) -> Result<app_domains::inference::models::Conversation, AppError> {
        self.domains
            .inference
            .new_conversation(ctx, &agent_name, &self.domains.skills)
            .await
    }

    pub async fn infer(
        &self,
        ctx: &AppContext,
        conversation: &mut app_domains::inference::models::Conversation,
    ) -> Result<(), AppError> {
        self.domains
            .inference
            .infer(ctx, conversation, self.domains.skills.as_ref())
            .await
    }
}
