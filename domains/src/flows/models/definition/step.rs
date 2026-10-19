// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Action, Condition, any_of, identifier, invalid, validate_list, within};
use crate::core::{
    models::{AppContext, AppError},
    traits::TemplateEngine,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Step {
    pub name: String,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub when: Option<Vec<Condition>>,
    #[serde(flatten)]
    pub action: Action,
}

impl Step {
    pub fn context_name(&self) -> &str {
        self.context.as_deref().unwrap_or_default()
    }

    /// Whether `when` allows the step to run; true without `when`.
    pub(crate) async fn runs(
        &self,
        ctx: &AppContext,
        engine: &(dyn TemplateEngine + Send + Sync),
        values: &Value,
    ) -> Result<bool, AppError> {
        match &self.when {
            Some(when) => any_of(ctx, engine, when, values).await,
            None => Ok(true),
        }
    }

    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        identifier(ctx, "step", &self.name)?;
        self.check(ctx)
            .map_err(|err| within(&format!("step '{}'", self.name), err))
    }

    fn check(&self, ctx: &AppContext) -> Result<(), AppError> {
        match (&self.action, &self.context) {
            (Action::Loop(_), Some(_)) => return Err(invalid(ctx, "a loop has no context")),
            (Action::Set(_), Some(_)) => return Err(invalid(ctx, "a set step has no context")),
            (Action::Flow(_), Some(_)) => {
                return Err(invalid(ctx, "a flow step has no context"));
            }
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
