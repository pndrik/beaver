// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::fmt::Display;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
};

pub(super) fn not_found(ctx: &AppContext, path: &str) -> AppError {
    app_error!(
        NotFound,
        "file_not_found",
        &format!("Path '{path}' does not exist"),
        ctx.clone()
    )
}

pub(super) fn exists(ctx: &AppContext, path: &str) -> AppError {
    app_error!(
        Conflict,
        "file_exists",
        &format!("Path '{path}' already exists"),
        ctx.clone()
    )
}

pub(super) fn escape(ctx: &AppContext, path: &str) -> AppError {
    app_error!(
        Validation,
        "access_violation",
        &format!("Path '{path}' is outside of the file system"),
        ctx.clone()
    )
}

pub(super) fn invalid(ctx: &AppContext, path: &str, reason: &str) -> AppError {
    app_error!(
        Validation,
        "file_system_invalid",
        &format!("Path '{path}' {reason}"),
        ctx.clone()
    )
}

pub(super) fn failed(ctx: &AppContext, action: &str, path: &str, error: impl Display) -> AppError {
    app_error!(
        Internal,
        "file_system_failed",
        &format!("Failed to {action} '{path}': {error}"),
        ctx.clone()
    )
}
