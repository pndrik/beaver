// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::Schema;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub schema: Schema,
}

impl Tool {
    pub fn new(name: String, description: String, schema: Schema) -> Self {
        Self {
            name,
            description,
            schema,
        }
    }
}
