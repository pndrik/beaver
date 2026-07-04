// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{fs, path::Path};

use super::{File, MAX_FILES_LISTED};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
};

impl File {
    pub(super) fn list_dir(
        ctx: &AppContext,
        path: &Path,
        recursive: bool,
        items: &mut Vec<String>,
    ) -> Result<(), AppError> {
        if path.is_dir() {
            for entry in fs::read_dir(path).map_err(|e| {
                app_error!(
                    Internal,
                    "skill_failed",
                    &format!("Failed to read directory '{}': {}", path.display(), e),
                    ctx.clone()
                )
            })? {
                let entry = entry.map_err(|e| {
                    app_error!(
                        Internal,
                        "skill_failed",
                        &format!(
                            "Failed to read directory entry in '{}': {}",
                            path.display(),
                            e
                        ),
                        ctx.clone()
                    )
                })?;
                let entry_path = entry.path();
                if recursive && entry_path.is_dir() {
                    Self::list_dir(ctx, &entry_path, recursive, items)?;
                    continue;
                }
                items.push(entry_path.to_string_lossy().to_string());

                if items.len() as u64 >= MAX_FILES_LISTED {
                    items.push("[truncated]".to_string());
                    break;
                }
            }

            return Ok(());
        }

        Err(app_error!(
            Validation,
            "skill_failed",
            &format!("Path '{}' is not a directory", path.display()),
            ctx.clone()
        ))
    }

    pub(super) fn create_dir(ctx: &AppContext, path: &Path) -> Result<(), AppError> {
        fs::create_dir_all(path).map_err(|e| {
            app_error!(
                Internal,
                "skill_failed",
                &format!("Failed to create directory '{}': {}", path.display(), e),
                ctx.clone()
            )
        })?;

        Ok(())
    }
}
