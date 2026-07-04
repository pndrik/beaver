// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::path::{Path, PathBuf};

use super::File;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    skills::models::SkillPermission,
};

impl File {
    pub(super) fn has_access_to_path(
        ctx: &AppContext,
        permissions: &SkillPermission,
        path: &str,
    ) -> bool {
        let path_cannocalized = match Path::new(&path).canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        }
        .to_string_lossy()
        .to_string();

        if permissions.scopes.len() > 0 {
            for scope in &permissions.scopes {
                let scope_str = format!(
                    "{}/{}",
                    ctx.chroot.trim_end_matches('/'),
                    scope.trim_start_matches('/').trim_end_matches('/')
                );

                let scope_canonicalized = match Path::new(&scope_str).canonicalize() {
                    Ok(p) => p,
                    Err(_) => continue,
                }
                .to_string_lossy()
                .to_string();

                if path_cannocalized.starts_with(&scope_canonicalized) {
                    return true;
                }
            }

            return false;
        }

        true
    }

    pub(super) fn sanitized_absolute_path(
        ctx: &AppContext,
        permissions: &SkillPermission,
        path: &str,
    ) -> Result<PathBuf, AppError> {
        let absolute_path = ctx.get_absolute_path(path)?;
        if !Self::has_access_to_path(ctx, permissions, &absolute_path) {
            return Err(app_error!(
                Validation,
                "skill_failed",
                &format!("Access denied to path '{}'", path),
                ctx.clone()
            ));
        }

        Path::new(&absolute_path).canonicalize().map_err(|e| {
            app_error!(
                Validation,
                "skill_failed",
                &format!("Invalid path '{}': {}", path, e),
                ctx.clone()
            )
        })
    }
}
