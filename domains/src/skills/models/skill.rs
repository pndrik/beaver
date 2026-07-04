// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::Schema;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub parameters: Schema,
}

impl Skill {
    pub fn new(name: String, description: String, parameters: Schema) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }
}
