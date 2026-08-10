// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::Model;
use crate::skills::models::SkillPermission;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Agent {
    pub metadata: AgentMetadata,
    pub permissions: AgentPermissions,

    #[serde(default, skip_serializing)]
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
    pub skills: Vec<SkillPermission>,
}

impl AgentPermissions {
    pub fn has_skill_permission(&self, skill_name: &str) -> bool {
        self.get_permission_for_skill(skill_name).is_some()
    }

    pub(crate) fn get_permission_for_skill(&self, skill_name: &str) -> Option<SkillPermission> {
        for permission in &self.skills {
            if permission.name.starts_with("^") && permission.name.ends_with("$") {
                let regex = match Regex::new(&permission.name.clone()) {
                    Ok(r) => r,
                    Err(_) => {
                        continue;
                    }
                };

                if regex.is_match(skill_name) {
                    return Some(permission.clone());
                }
            }

            if permission.name == skill_name {
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
        permissions_skills: Vec<SkillPermission>,
    ) -> Self {
        Self {
            metadata: AgentMetadata {
                name,
                display_name,
                description,
                model,
            },
            permissions: AgentPermissions {
                skills: permissions_skills,
            },
            prompt,
        }
    }

    pub fn model(&self) -> &Model {
        &self.metadata.model.name
    }
}
