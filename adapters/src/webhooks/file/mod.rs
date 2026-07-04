// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{models::Webhook, traits::WebhookProvider},
};

mod file;
mod parse;

const CONFIGURATION_WEBHOOKS_DIR: &str = "webhooks.path";

pub struct File {}

impl File {
    pub fn new(_ctx: &AppContext) -> Self {
        Self {}
    }

    async fn get_webhooks_dir(&self, ctx: &AppContext) -> Result<String, AppError> {
        ctx.configuration
            .get_string(ctx, CONFIGURATION_WEBHOOKS_DIR)
            .await
            .map(|path| path.trim_end_matches('/').to_string())
    }

    fn load_webhook(
        &self,
        ctx: &AppContext,
        webhooks_dir: &str,
        webhook_name: &str,
    ) -> Result<Webhook, AppError> {
        let config_content =
            file::read_file(ctx, &format!("{}/{}.yaml", webhooks_dir, webhook_name))?;
        let webhook = parse::parse_webhook(ctx, &config_content)?;

        if webhook.metadata.name.is_empty() {
            return Err(app_error!(
                Internal,
                "configuration_load_failed",
                "Webhook name is missing.",
                ctx.clone()
            ));
        }

        if webhook.metadata.name != webhook_name {
            return Err(app_error!(
                Internal,
                "configuration_load_failed",
                &format!(
                    "Webhook name mismatch: expected '{}', found '{}'.",
                    webhook_name, webhook.metadata.name
                ),
                ctx.clone()
            ));
        }

        Ok(webhook)
    }
}

#[async_trait]
impl WebhookProvider for File {
    async fn reload(&self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, ctx: &AppContext) -> Result<Vec<Webhook>, AppError> {
        let webhooks_dir = self.get_webhooks_dir(ctx).await?;
        let mut valid_webhooks = Vec::new();
        let webhooks = file::list_yaml_files(ctx, &webhooks_dir)?;

        for webhook in webhooks {
            let webhook_name = webhook
                .split('/')
                .last()
                .and_then(|name| name.strip_suffix(".yaml"))
                .ok_or_else(|| {
                    app_error!(
                        Internal,
                        "configuration_load_failed",
                        &format!("Invalid webhook file name: {}", webhook),
                        ctx.clone()
                    )
                })?;

            let webhook = self.load_webhook(ctx, &webhooks_dir, &webhook_name)?;
            valid_webhooks.push(webhook);
        }

        Ok(valid_webhooks)
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Webhook, AppError> {
        if !file::file_exists(&format!(
            "{}/{}.yaml",
            self.get_webhooks_dir(ctx).await?,
            name
        )) {
            return Err(app_error!(
                NotFound,
                "webhook_not_found",
                &format!("Webhook '{}' not found.", name),
                ctx.clone()
            ));
        }

        let agents_dir = self.get_webhooks_dir(ctx).await?;
        self.load_webhook(ctx, &agents_dir, name)
    }
}
