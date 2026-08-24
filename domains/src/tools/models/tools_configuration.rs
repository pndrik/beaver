// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ToolsConfiguration {
    #[serde(default)]
    pub toolsets: Vec<ToolSet>,
    #[serde(default)]
    pub permissions: Vec<ToolPermission>,
}

impl ToolsConfiguration {
    pub fn has_tool_permission(&self, tool_name: &str) -> bool {
        self.get_permission_for_tool(tool_name).is_some()
    }

    pub(crate) fn get_permission_for_tool(&self, tool_name: &str) -> Option<ToolPermission> {
        for permission in &self.permissions {
            if permission.name.starts_with("^") && permission.name.ends_with("$") {
                let regex = match Regex::new(&permission.name.clone()) {
                    Ok(r) => r,
                    Err(_) => {
                        continue;
                    }
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
