// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{models::Webhook, traits::WebhookProvider},
};

/// Serves a fixed list of webhooks held in memory.
pub struct Memory {
    webhooks: Vec<Webhook>,
}

impl Memory {
    pub fn new(webhooks: Vec<Webhook>) -> Self {
        Self { webhooks }
    }
}

#[async_trait]
impl WebhookProvider for Memory {
    async fn reload(&self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, _ctx: &AppContext) -> Result<Vec<Webhook>, AppError> {
        Ok(self.webhooks.clone())
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Webhook, AppError> {
        self.webhooks
            .iter()
            .find(|webhook| webhook.metadata.name == name)
            .cloned()
            .ok_or_else(|| {
                app_error!(
                    NotFound,
                    "webhook_not_found",
                    &format!("Webhook '{}' not found.", name),
                    ctx.clone()
                )
            })
    }
}
