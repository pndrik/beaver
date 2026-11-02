// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use super::{Action, Argument, Context, Step, Triggers, identifier, invalid, within};
use crate::core::models::{AppContext, AppError};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    #[serde(default)]
    pub arguments: BTreeMap<String, Argument>,
    #[serde(default)]
    pub contexts: BTreeMap<String, Context>,
    #[serde(default)]
    pub triggers: Triggers,
    pub steps: Vec<Step>,
}

impl Spec {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        if self.steps.is_empty() {
            return Err(invalid(ctx, "spec.steps is empty"));
        }

        for (name, context) in &self.contexts {
            identifier(ctx, "context", name)?;
            context
                .validate(ctx)
                .map_err(|err| within(&format!("context '{name}'"), err))?;
        }

        for (name, argument) in &self.arguments {
            identifier(ctx, "argument", name)?;
            argument
                .validate(ctx)
                .map_err(|err| within(&format!("argument '{name}'"), err))?;
        }

        self.triggers.validate(ctx, &self.arguments)?;

        let mut step_names = HashSet::new();
        let mut used_contexts = HashSet::new();
        self.check_steps(ctx, &self.steps, None, &mut step_names, &mut used_contexts)?;

        if let Some(name) = self
            .contexts
            .keys()
            .find(|name| !used_contexts.contains(name.as_str()))
        {
            return Err(invalid(
                ctx,
                &format!("context '{name}' is not used by any step"),
            ));
        }

        Ok(())
    }

    fn check_steps<'a>(
        &'a self,
        ctx: &AppContext,
        steps: &'a [Step],
        parent: Option<&str>,
        step_names: &mut HashSet<&'a str>,
        used_contexts: &mut HashSet<&'a str>,
    ) -> Result<(), AppError> {
        for (index, step) in steps.iter().enumerate() {
            let position = Step::position(parent, index);
            step.validate(ctx, &position)?;

            if let Some(name) = &step.name
                && !step_names.insert(name.as_str())
            {
                return Err(invalid(
                    ctx,
                    &format!("step name '{name}' is used more than once"),
                ));
            }

            if let Some(context) = &step.context {
                if !self.contexts.contains_key(context) {
                    return Err(invalid(
                        ctx,
                        &format!(
                            "step '{}' uses unknown context '{}'",
                            step.label(&position),
                            context
                        ),
                    ));
                }
                used_contexts.insert(context.as_str());
            }

            if let Action::Loop(arguments) = &step.action {
                self.check_steps(
                    ctx,
                    &arguments.steps,
                    Some(&position),
                    step_names,
                    used_contexts,
                )?;
            }
        }
        Ok(())
    }
}
