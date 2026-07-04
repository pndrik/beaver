// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use yaml_serde;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::models::Webhook,
};

pub(super) fn parse_webhook(ctx: &AppContext, file_content: &str) -> Result<Webhook, AppError> {
    yaml_serde::from_str(file_content).map_err(|e| {
        app_error!(
            Internal,
            "configuration_load_failed",
            &format!("Failed to parse webhook file: {}", e),
            ctx.clone()
        )
    })
}
