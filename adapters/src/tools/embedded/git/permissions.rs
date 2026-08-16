// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::path::{Path, PathBuf};

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::models::ToolPermission,
};

fn validate_path(
    ctx: &AppContext,
    _permissions: &ToolPermission,
    path: &str,
) -> Result<(), AppError> {
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

    Ok(())
}

pub(super) fn existing_repo_path(
    ctx: &AppContext,
    permissions: &ToolPermission,
    path: &str,
) -> Result<PathBuf, AppError> {
    validate_path(ctx, permissions, path)?;

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

pub(super) fn clone_target_path(
    ctx: &AppContext,
    permissions: &ToolPermission,
    path: &str,
) -> Result<PathBuf, AppError> {
    validate_path(ctx, permissions, path)?;

    let chroot_root = Path::new(&ctx.chroot);
    let sub_path_sanitized = path.trim_start_matches('/');
    let full_path = chroot_root.join(sub_path_sanitized);

    if full_path.exists() {
        return Err(app_error!(
            Validation,
            "tool_failed",
            &format!("Path '{}' already exists", path),
            ctx.clone()
        ));
    }

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!("Failed to create parent directory for '{}': {}", path, e),
                ctx.clone()
            )
        })?;
    }

    Ok(full_path)
}
