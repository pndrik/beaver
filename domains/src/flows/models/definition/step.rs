// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{Action, Condition, identifier, invalid, validate_list, within};
use crate::core::models::{AppContext, AppError};

/// `type` and `arguments` of the action sit next to `name` and `context`.
/// `flatten` rules out `deny_unknown_fields` here; the argument structs
/// still reject unknown fields.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Step {
    pub name: String,
    /// Required for agent and tool steps, forbidden for loops.
    #[serde(default)]
    pub context: Option<String>,
    /// The step runs if any item is true, or if `when` is missing.
    #[serde(default)]
    pub when: Option<Vec<Condition>>,
    #[serde(flatten)]
    pub action: Action,
}

impl Step {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        identifier(ctx, "step", &self.name)?;
        self.check(ctx)
            .map_err(|err| within(&format!("step '{}'", self.name), err))
    }

    fn check(&self, ctx: &AppContext) -> Result<(), AppError> {
        match (&self.action, &self.context) {
            (Action::Loop(_), Some(_)) => return Err(invalid(ctx, "a loop has no context")),
            (Action::Agent(_) | Action::Tool(_), None) => {
                return Err(invalid(ctx, "context is missing"));
            }
            _ => {}
        }
        if let Some(when) = &self.when {
            validate_list(ctx, "when", when)?;
        }
        self.action.validate(ctx)
    }
}
