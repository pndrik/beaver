// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::invalid;
use crate::core::models::{AppContext, AppError};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsTool {
    pub tool: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl ArgumentsTool {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        if self.tool.is_empty() {
            return Err(invalid(ctx, "arguments.tool is empty"));
        }
        Ok(())
    }
}
