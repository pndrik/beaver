// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::path::Path;

use crate::file_system::errors::invalid;
use app_domains::core::models::{AppContext, AppError, file};

pub(crate) trait AsBeaverFilePath {
    fn as_beaver_file_path(&self, ctx: &AppContext) -> Result<file::Path, AppError>;
}
impl AsBeaverFilePath for Path {
    fn as_beaver_file_path(&self, ctx: &AppContext) -> Result<file::Path, AppError> {
        let Some(path) = self.to_str() else {
            return Err(invalid(ctx, &self.to_string_lossy(), "is not valid UTF-8"));
        };
        match path {
            "" | "." => Ok(file::Path::root()),
            path => file::Path::parse(ctx, &format!("/{path}")),
        }
    }
}
