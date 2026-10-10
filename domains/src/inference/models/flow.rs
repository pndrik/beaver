// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::{
    app_error,
    core::models::{AppContext, AppError},
};

const STEP_NAME_PATTERN: &str = r"^[A-Za-z_][A-Za-z0-9_]*$";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Flow {
    pub metadata: FlowMetadata,
    pub spec: FlowSpec,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlowMetadata {
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlowSpec {
    #[serde(default)]
    pub triggers: FlowTriggers,
    pub steps: Vec<FlowStep>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FlowTriggers {
    #[serde(default)]
    pub webhook: Option<FlowTriggerWebhook>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlowTriggerWebhook {
    pub name: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlowStep {
    pub name: String,
    pub context: FlowContext,
    pub action: FlowAction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlowContext {
    pub name: String,
    #[serde(default)]
    pub chroot: Option<FlowChroot>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlowChroot {
    pub path: String,
    #[serde(default)]
    pub create_if_missing: bool,
    #[serde(default)]
    pub delete: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "arguments", rename_all = "snake_case")]
pub enum FlowAction {
    Agent(FlowAgentArguments),
    Tool(FlowToolArguments),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlowAgentArguments {
    pub agent: String,
    pub prompt: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlowToolArguments {
    pub tool: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
    #[serde(default)]
    pub scopes: Vec<String>,
}

fn invalid(ctx: &AppContext, flow_name: &str, message: &str) -> AppError {
    app_error!(
        Validation,
        "flow_invalid",
        &format!("Flow '{}' is invalid: {}", flow_name, message),
        ctx.clone()
    )
}

impl Flow {
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

            if let FlowAction::Agent(arguments) = &step.action {
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

        Ok(())
    }
}
