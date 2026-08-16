// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::path::{Path, PathBuf};

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::models::ToolPermission,
};

pub(super) fn check_scope(permissions: &ToolPermission, path: &str) -> bool {
    if permissions.scopes.is_empty() {
        return true;
    }

    for scope in &permissions.scopes {
        if path.starts_with(scope) {
            return true;
        }
    }

    false
}

pub(super) fn absolute_path(
    ctx: &AppContext,
    permissions: &ToolPermission,
    path: &str,
) -> Result<PathBuf, AppError> {
    if path.contains("..") {
        return Err(app_error!(
            Validation,
            "tool_failed",
            &format!("Path must not contain '..': '{}'", path),
            ctx.clone()
        ));
    }

    if !path.starts_with('/') {
        return Err(app_error!(
            Validation,
            "tool_failed",
            &format!("Path must be absolute: '{}'", path),
            ctx.clone()
        ));
    }

    if !check_scope(permissions, path) {
        return Err(app_error!(
            Unauthorized,
            "tool_failed",
            &format!(
                "Path '{}' is not within allowed scopes: {:?}",
                path, permissions.scopes
            ),
            ctx.clone()
        ));
    }

    let absolute_path = ctx.get_absolute_path(path)?;
    Path::new(&absolute_path).canonicalize().map_err(|e| {
        app_error!(
            Validation,
            "tool_failed",
            &format!("Invalid path '{}': {}", path, e),
            ctx.clone()
        )
    })
}
