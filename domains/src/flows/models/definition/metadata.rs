// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: String,
}

impl Metadata {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("metadata.name is missing".to_string());
        }
        Ok(())
    }
}
