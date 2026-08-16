// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::fs;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
};

pub(super) fn list_directories(ctx: &AppContext, path: &str) -> Result<Vec<String>, AppError> {
    Ok(fs::read_dir(path)
        .map_err(|_| {
            app_error!(
                Internal,
                "configuration_load_failed",
                &format!("Failed to list directory: {}", path),
                ctx.clone()
            )
        })?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .map(|p| p.display().to_string())
        .collect())
}

pub(super) fn read_file(ctx: &AppContext, path: &str) -> Result<String, AppError> {
    fs::read_to_string(path).map_err(|e| {
        app_error!(
            Internal,
            "configuration_load_failed",
            &format!("Failed to read file {}: {}", path, e),
            ctx.clone()
        )
    })
}
