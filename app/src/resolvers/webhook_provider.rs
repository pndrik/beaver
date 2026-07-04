// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::webhooks::File;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::traits::WebhookProvider,
};

const CONFIGURATION_WEBHOOKS_PROVIDER: &str = "webhooks.provider";

async fn get_provider_name(ctx: &AppContext) -> Result<String, AppError> {
    ctx.configuration
        .get_string(ctx, CONFIGURATION_WEBHOOKS_PROVIDER)
        .await
}

pub async fn webhook_provider(
    ctx: &AppContext,
) -> Result<Arc<dyn WebhookProvider + Send + Sync>, AppError> {
    let provider_name = get_provider_name(&ctx).await?;
    if provider_name != "file" {
        return Err(app_error!(
            Internal,
            "bootstrap_failed",
            &format!(
                "Unsupported webhook provider: {}. Only 'file' is supported.",
                provider_name
            ),
            ctx.clone()
        ));
    }

    Ok(Arc::new(File::new(ctx)))
}
