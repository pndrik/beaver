// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::invalid;
use crate::core::models::{AppContext, AppError};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsAgent {
    pub prompt: String,
}

impl ArgumentsAgent {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        if self.prompt.is_empty() {
            return Err(invalid(ctx, "arguments.prompt is empty"));
        }
        Ok(())
    }
}
