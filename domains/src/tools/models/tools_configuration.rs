// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    app_error,
    core::models::{AppContext, AppError, file},
};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ToolsConfiguration {
    #[serde(default)]
    pub toolsets: Vec<ToolSet>,
    #[serde(default)]
    pub permissions: Vec<ToolPermission>,
    /// Offers provider tools an optional `harness_redirect_to_file` argument.
    #[serde(default)]
    pub redirect_output: bool,
}

impl ToolsConfiguration {
    pub fn has_tool_permission(&self, tool_name: &str) -> bool {
        self.get_permission_for_tool(tool_name).is_some()
    }

    pub(crate) fn get_permission_for_tool(&self, tool_name: &str) -> Option<ToolPermission> {
        for permission in &self.permissions {
            if permission.name.starts_with('^') && permission.name.ends_with('$') {
                let Ok(regex) = Regex::new(&permission.name) else {
                    continue;
                };

                if regex.is_match(tool_name) {
                    return Some(permission.clone());
                }
            }

            if permission.name == tool_name {
                return Some(permission.clone());
            }
        }

        None
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolSet {
    pub name: String,
    pub autoload: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolPermission {
    pub name: String,
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl ToolPermission {
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
