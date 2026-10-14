// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

mod action;
pub use action::*;

mod agent_arguments;
pub use agent_arguments::*;

mod chroot;
pub use chroot::*;

mod context;
pub use context::*;

mod metadata;
pub use metadata::*;

mod spec;
pub use spec::*;

mod step;
pub use step::*;

mod tool_arguments;
pub use tool_arguments::*;

mod trigger_webhook;
pub use trigger_webhook::*;

mod triggers;
pub use triggers::*;

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::{
    app_error,
    core::models::{AppContext, AppError},
};

const STEP_NAME_PATTERN: &str = r"^[A-Za-z_][A-Za-z0-9_]*$";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Definition {
    pub metadata: Metadata,
    pub spec: Spec,
}

fn invalid(ctx: &AppContext, flow_name: &str, message: &str) -> AppError {
    app_error!(
        Validation,
        "flow_invalid",
        &format!("Flow '{}' is invalid: {}", flow_name, message),
        ctx.clone()
    )
}

impl Definition {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        let name = &self.metadata.name;
        if name.is_empty() {
            return Err(invalid(ctx, name, "metadata.name is missing"));
        }

        if self.spec.steps.is_empty() {
            return Err(invalid(ctx, name, "spec.steps is empty"));
        }

        let step_name_pattern = Regex::new(STEP_NAME_PATTERN).expect("valid step name pattern");
        let mut step_names = HashSet::new();
        let mut context_agents: HashMap<&str, &str> = HashMap::new();

        for step in &self.spec.steps {
            if !step_name_pattern.is_match(&step.name) {
                return Err(invalid(
                    ctx,
                    name,
                    &format!("step name '{}' must match {}", step.name, STEP_NAME_PATTERN),
                ));
            }

            if !step_names.insert(step.name.as_str()) {
                return Err(invalid(
                    ctx,
                    name,
                    &format!("step name '{}' is used more than once", step.name),
                ));
            }

            if step.context.name.is_empty() {
                return Err(invalid(
                    ctx,
                    name,
                    &format!("step '{}' has no context name", step.name),
                ));
            }

            if let Action::Agent(arguments) = &step.action {
                match context_agents.get(step.context.name.as_str()) {
                    Some(agent) if *agent != arguments.agent => {
                        return Err(invalid(
                            ctx,
                            name,
                            &format!(
                                "context '{}' can not be used by different agents",
                                step.context.name
                            ),
                        ));
                    }
                    Some(_) => {}
                    None => {
                        context_agents.insert(&step.context.name, &arguments.agent);
                    }
                }
            }
        }

        for step in &self.spec.steps {
            if !context_agents.contains_key(step.context.name.as_str()) {
                return Err(invalid(
                    ctx,
                    name,
                    &format!("context '{}' has no agent step", step.context.name),
                ));
            }
        }

        Ok(())
    }

    /// The agent of the first agent step on `context`.
    pub fn context_agent(&self, context: &str) -> Option<&str> {
        self.spec.steps.iter().find_map(|step| match &step.action {
            Action::Agent(arguments) if step.context.name == context => {
                Some(arguments.agent.as_str())
            }
            _ => None,
        })
    }
}
