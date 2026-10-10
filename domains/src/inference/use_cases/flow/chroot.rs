// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{
    app_error,
    core::models::{AppContext, AppError},
};

fn invalid(ctx: &AppContext, message: String) -> AppError {
    app_error!(Validation, "access_violation", &message, ctx.clone())
}

/// Resolves `.` and `..` without touching the file system.
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

/// Canonicalizes the longest existing ancestor of `path` (following symlinks)
/// and appends the components that don't exist yet.
fn resolve_existing_prefix(path: &Path) -> std::io::Result<PathBuf> {
    let mut existing = path.to_path_buf();
    let mut missing = Vec::new();
    while !existing.exists() {
        match (existing.file_name(), existing.parent()) {
            (Some(name), Some(parent)) => {
                missing.push(name.to_os_string());
                existing = parent.to_path_buf();
            }
            _ => break,
        }
    }

    let mut resolved = existing.canonicalize()?;
    for name in missing.into_iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

pub(super) fn global_chroot(ctx: &AppContext) -> Result<PathBuf, AppError> {
    Path::new(&ctx.chroot).canonicalize().map_err(|e| {
        invalid(
            ctx,
            format!("Invalid global chroot '{}': {}", ctx.chroot, e),
        )
    })
}

pub(super) fn prepare(
    ctx: &AppContext,
    path: &str,
    create_if_missing: bool,
    delete: bool,
) -> Result<PathBuf, AppError> {
    let global = global_chroot(ctx)?;
    let requested = Path::new(path);
    let joined = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        global.join(requested)
    };

    let outside = || {
        invalid(
            ctx,
            format!("Chroot '{}' is outside of the global chroot", path),
        )
    };
    let resolved = resolve_existing_prefix(&normalize(&joined))
        .map_err(|e| invalid(ctx, format!("Invalid chroot '{}': {}", path, e)))?;
    if !resolved.starts_with(&global) {
        return Err(outside());
    }

    if !resolved.exists() {
        if !create_if_missing {
            return Err(invalid(ctx, format!("Chroot '{}' does not exist", path)));
        }
        fs::create_dir_all(&resolved)
            .map_err(|e| invalid(ctx, format!("Failed to create chroot '{}': {}", path, e)))?;
    }

    let canonical = resolved
        .canonicalize()
        .map_err(|e| invalid(ctx, format!("Invalid chroot '{}': {}", path, e)))?;
    if !canonical.starts_with(&global) {
        return Err(outside());
    }
    if !canonical.is_dir() {
        return Err(invalid(
            ctx,
            format!("Chroot '{}' is not a directory", path),
        ));
    }
    if delete && canonical == global {
        return Err(invalid(
            ctx,
            format!(
                "Chroot '{}' is the global chroot and can't be deleted",
                path
            ),
        ));
    }

    Ok(canonical)
}

/// The chroot relative to the global chroot, `.` for the global chroot itself.
pub(super) fn relative(global: &Path, chroot: &Path) -> String {
    match chroot.strip_prefix(global) {
        Ok(rel) if rel.as_os_str().is_empty() => ".".to_string(),
        Ok(rel) => rel.to_string_lossy().to_string(),
        Err(_) => chroot.to_string_lossy().to_string(),
    }
}

pub(super) async fn cleanup(ctx: &AppContext, chroot: &Path) {
    let is_global = global_chroot(ctx).map_or(true, |global| global == chroot);
    if is_global || !chroot.exists() {
        return;
    }

    if let Err(e) = fs::remove_dir_all(chroot) {
        ctx.logger
            .warn(
                ctx,
                &format!("Failed to delete chroot '{}': {}", chroot.display(), e),
            )
            .await;
    }
}
