// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{Condition, Step, invalid, validate_list};
use crate::core::models::{AppContext, AppError};

const LOOP_MAX_LIMIT: u32 = 100;

fn default_max() -> u32 {
    3
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ArgumentsLoop {
    pub steps: Vec<Step>,
    #[serde(default = "default_max")]
    pub max: u32,
    #[serde(default)]
    pub until: Option<Vec<Condition>>,
}

impl ArgumentsLoop {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        if !(1..=LOOP_MAX_LIMIT).contains(&self.max) {
            return Err(invalid(
                ctx,
                &format!("arguments.max must be between 1 and {}", LOOP_MAX_LIMIT),
            ));
        }
        if self.steps.is_empty() {
            return Err(invalid(ctx, "arguments.steps is empty"));
        }
        match &self.until {
            Some(until) => validate_list(ctx, "arguments.until", until),
            None => Ok(()),
        }
    }
}
