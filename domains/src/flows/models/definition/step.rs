// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{Action, identifier};

/// `type` and `arguments` of the action sit next to `name` and `context`.
/// `flatten` rules out `deny_unknown_fields` here; the argument structs
/// still reject unknown fields.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Step {
    pub name: String,
    pub context: String,
    #[serde(flatten)]
    pub action: Action,
}

impl Step {
    pub fn validate(&self) -> Result<(), String> {
        identifier("step", &self.name)?;
        self.action
            .validate()
            .map_err(|e| format!("step '{}': {}", self.name, e))
    }
}
