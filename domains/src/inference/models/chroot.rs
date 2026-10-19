// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::path::{Component, Path, PathBuf};

use crate::{
    app_error,
    core::models::{AppContext, AppError},
};

fn canonical_chroot(ctx: &AppContext, chroot: &str) -> Result<PathBuf, AppError> {
    Path::new(chroot).canonicalize().map_err(|e| {
        app_error!(
            Validation,
            "access_violation",
            &format!("Invalid chroot '{}': {}", chroot, e),
            ctx.clone()
        )
    })
}

pub fn resolve_path(ctx: &AppContext, chroot: &str, path: &str) -> Result<String, AppError> {
    let joined = format!(
        "{}/{}",
        chroot.trim_end_matches('/'),
        path.trim_start_matches('/')
    );

    let canonicalized = Path::new(&joined).canonicalize().map_err(|e| {
        app_error!(
            Validation,
            "access_violation",
            &format!("Invalid path '{}': {}", path, e),
            ctx.clone()
        )
    })?;

    if !canonicalized.starts_with(canonical_chroot(ctx, chroot)?) {
        return Err(app_error!(
            Validation,
            "access_violation",
            &format!("Path '{}' is outside of chroot", path),
            ctx.clone()
        ));
    }

    if !canonicalized.exists() {
        return Err(app_error!(
            Validation,
            "access_violation",
            &format!("Path '{}' does not exist", path),
            ctx.clone()
        ));
    }

    Ok(canonicalized.to_string_lossy().to_string())
}

pub fn resolve_new_path(ctx: &AppContext, chroot: &str, path: &str) -> Result<String, AppError> {
    let virtual_path = Path::new(path);
    if !virtual_path.has_root() {
        return Err(app_error!(
            Validation,
            "access_violation",
            &format!("Path must be absolute: '{}'", path),
            ctx.clone()
        ));
    }

    if virtual_path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(app_error!(
            Validation,
            "access_violation",
            &format!("Path must not contain '..': '{}'", path),
            ctx.clone()
        ));
    }

    let (Some(parent), Some(file_name)) = (virtual_path.parent(), virtual_path.file_name()) else {
        return Err(app_error!(
            Validation,
            "access_violation",
            &format!("Path '{}' has no file name", path),
            ctx.clone()
        ));
    };

    let parent = PathBuf::from(resolve_path(ctx, chroot, &parent.to_string_lossy())?);
    if !parent.is_dir() {
        return Err(app_error!(
            Validation,
            "access_violation",
            &format!("Parent of '{}' is not a directory", path),
            ctx.clone()
        ));
    }

    let target = parent.join(file_name);
    if target.symlink_metadata().is_err() {
        return Ok(target.to_string_lossy().to_string());
    }

    let canonicalized = target.canonicalize().map_err(|e| {
        app_error!(
            Validation,
            "access_violation",
            &format!("Invalid path '{}': {}", path, e),
            ctx.clone()
        )
    })?;
    if !canonicalized.starts_with(canonical_chroot(ctx, chroot)?) {
        return Err(app_error!(
            Validation,
            "access_violation",
            &format!("Path '{}' is outside of chroot", path),
            ctx.clone()
        ));
    }

    if canonicalized.is_dir() {
        return Err(app_error!(
            Validation,
            "access_violation",
            &format!("Path '{}' is a directory", path),
            ctx.clone()
        ));
    }

    Ok(canonicalized.to_string_lossy().to_string())
}
