// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::models::ToolPermission,
};

const WEB_SCOPE: &str = "web:";

pub(super) fn web_patterns(
    ctx: &AppContext,
    permissions: &ToolPermission,
) -> Result<Vec<Regex>, AppError> {
    permissions
        .scopes
        .iter()
        .filter_map(|scope| scope.strip_prefix(WEB_SCOPE))
        .map(|pattern| {
            Regex::new(pattern).map_err(|e| {
                app_error!(
                    Validation,
                    "tool_failed",
                    &format!("Invalid web scope '{}': {}", pattern, e),
                    ctx.clone()
                )
            })
        })
        .collect()
}
