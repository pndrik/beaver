// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{ArgumentsAgent, ArgumentsFlow, ArgumentsLoop, ArgumentsSet, ArgumentsTool};
use crate::core::models::{AppContext, AppError};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "arguments", rename_all = "snake_case")]
pub enum Action {
    Agent(ArgumentsAgent),
    Tool(ArgumentsTool),
    Loop(ArgumentsLoop),
    Set(ArgumentsSet),
    Flow(ArgumentsFlow),
}

impl Action {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        match self {
            Action::Agent(arguments) => arguments.validate(ctx),
            Action::Tool(arguments) => arguments.validate(ctx),
            Action::Loop(arguments) => arguments.validate(ctx),
            Action::Set(arguments) => arguments.validate(ctx),
            Action::Flow(arguments) => arguments.validate(ctx),
        }
    }
}
