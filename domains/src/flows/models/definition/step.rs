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
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub when: Option<Vec<Condition>>,
    #[serde(flatten)]
    pub action: Action,
}

impl Step {
    pub(crate) fn position(parent: Option<&str>, index: usize) -> String {
        match parent {
            Some(parent) => format!("{parent}.steps[{index}]"),
            None => format!("steps[{index}]"),
        }
    }

    pub fn label<'a>(&'a self, position: &'a str) -> &'a str {
        self.name.as_deref().unwrap_or(position)
    }

    pub fn context_name(&self) -> &str {
        self.context.as_deref().unwrap_or_default()
    }

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

    pub fn validate(&self, ctx: &AppContext, position: &str) -> Result<(), AppError> {
        if let Some(name) = &self.name {
            identifier(ctx, "step", name)?;
        }
        self.check(ctx)
            .map_err(|err| within(&format!("step '{}'", self.label(position)), err))
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
