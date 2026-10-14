// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::flows::File;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    flows::traits::FlowProvider,
};

const CONFIGURATION_FLOWS_PROVIDER: &str = "flows.provider";

async fn get_provider_name(ctx: &AppContext) -> Result<String, AppError> {
    ctx.configuration
        .get_string(ctx, CONFIGURATION_FLOWS_PROVIDER)
        .await
}

pub async fn flow_provider(
    ctx: &AppContext,
) -> Result<Arc<dyn FlowProvider + Send + Sync>, AppError> {
    let provider_name = get_provider_name(ctx).await?;
    if provider_name != "file" {
        return Err(app_error!(
            Internal,
            "bootstrap_failed",
            &format!(
                "Unsupported flow provider: {}. Only 'file' is supported.",
                provider_name
            ),
            ctx.clone()
        ));
    }

    Ok(Arc::new(File::new(ctx)))
}
