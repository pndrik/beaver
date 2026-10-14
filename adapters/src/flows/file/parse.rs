// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use yaml_serde;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    flows::models::Definition,
};

pub(super) fn parse_flow(ctx: &AppContext, file_content: &str) -> Result<Definition, AppError> {
    yaml_serde::from_str(file_content).map_err(|e| {
        app_error!(
            Validation,
            "flow_invalid",
            &format!("Failed to parse flow file: {}", e),
            ctx.clone()
        )
    })
}
