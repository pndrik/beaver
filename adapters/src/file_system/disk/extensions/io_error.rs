// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::io::{Error, ErrorKind};

use crate::file_system::errors::*;
use app_domains::core::models::{AppContext, AppError, file};

pub(crate) trait IsEscaped {
    fn is_escaped(&self) -> bool;
}
impl IsEscaped for Error {
    fn is_escaped(&self) -> bool {
        self.kind() == ErrorKind::PermissionDenied && self.raw_os_error().is_none()
    }
}

pub(crate) trait AsBeaverError {
    fn as_beaver_error(&self, ctx: &AppContext, path: &file::Path) -> AppError;
}
impl AsBeaverError for Error {
    fn as_beaver_error(&self, ctx: &AppContext, path: &file::Path) -> AppError {
        let path = path.as_str();
        match self.kind() {
            _ if self.is_escaped() => escape(ctx, path),
            ErrorKind::NotFound => not_found(ctx, path),
            ErrorKind::AlreadyExists => exists(ctx, path),
            ErrorKind::IsADirectory => invalid(ctx, path, "is a directory"),
            ErrorKind::NotADirectory => invalid(ctx, path, "is not a directory"),
            ErrorKind::DirectoryNotEmpty => invalid(ctx, path, "is not empty"),
            _ => failed(ctx, "access", path, self),
        }
    }
}
