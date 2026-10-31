// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::logging::Stdout;
use app_domains::{
    app_error,
    core::{
        models::{AppContext, AppError},
        traits::Logging,
    },
};

const CONFIGURATION_LOGGER_PROVIDER: &str = "logger.provider";

pub async fn logging(ctx: &AppContext) -> Result<Arc<dyn Logging + Send + Sync>, AppError> {
    let provider_name = ctx
        .configuration()
        .get_string(ctx, CONFIGURATION_LOGGER_PROVIDER)
        .await?;
    if provider_name != "stdout" {
        return Err(app_error!(
            Internal,
            "bootstrap_failed",
            &format!(
                "Unsupported logger provider: {}. Only 'stdout' is supported.",
                provider_name
            ),
            ctx.clone()
        ));
    }

    Ok(Arc::new(Stdout::new()))
}
