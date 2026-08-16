// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::Model;
use crate::tools::models::ToolPermission;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Agent {
    pub metadata: AgentMetadata,
    pub permissions: AgentPermissions,

    #[serde(default)]
    pub prompt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentMetadata {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub model: AgentMetadataModel,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentMetadataModel {
    pub name: Model,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentPermissions {
    pub tools: Vec<ToolPermission>,
}

impl AgentPermissions {
    pub fn has_tool_permission(&self, tool_name: &str) -> bool {
        self.get_permission_for_tool(tool_name).is_some()
    }

    pub(crate) fn get_permission_for_tool(&self, tool_name: &str) -> Option<ToolPermission> {
        for permission in &self.tools {
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

impl Agent {
    pub fn new(
        name: String,
        display_name: String,
        description: String,
        prompt: String,
        model: AgentMetadataModel,
        permissions_tools: Vec<ToolPermission>,
    ) -> Self {
        Self {
            metadata: AgentMetadata {
                name,
                display_name,
                description,
                model,
            },
            permissions: AgentPermissions {
                tools: permissions_tools,
            },
            prompt,
        }
    }

    pub fn model(&self) -> &Model {
        &self.metadata.model.name
    }
}
