// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{check_name, matches};
use crate::{
    app_error,
    core::models::{AppContext, AppError, file},
    inference::models::invalid,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ToolPermission {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl ToolPermission {
    pub fn matches(&self, tool_name: &str) -> bool {
        matches(&self.name, tool_name)
    }

    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        check_name(ctx, &self.name)?;
        if let Some(index) = self.scopes.iter().position(String::is_empty) {
            return Err(invalid(ctx, &format!("scope {index} is empty")));
        }
        Ok(())
    }

    pub fn scoped_path(&self, ctx: &AppContext, path: &str) -> Result<file::Path, AppError> {
        let path = file::Path::parse(ctx, path)?;
        let covered = self.scopes.is_empty()
            || self.scopes.iter().any(|scope| {
                file::Path::parse(ctx, scope).is_ok_and(|scope| path.starts_with(&scope))
            });
        if covered {
            Ok(path)
        } else {
            Err(app_error!(
                Unauthorized,
                "tool_failed",
                &format!(
                    "Path '{}' is not within allowed scopes: {:?}",
                    path, self.scopes
                ),
                ctx.clone()
            ))
        }
    }
}
