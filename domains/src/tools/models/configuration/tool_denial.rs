// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{check_name, matches};
use crate::core::models::{AppContext, AppError};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ToolDenial {
    pub name: String,
}

impl ToolDenial {
    pub fn matches(&self, tool_name: &str) -> bool {
        matches(&self.name, tool_name)
    }

    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        check_name(ctx, &self.name)
    }
}
