// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{AgentArguments, ToolArguments};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "arguments", rename_all = "snake_case")]
pub enum Action {
    Agent(AgentArguments),
    Tool(ToolArguments),
}

impl Action {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Action::Agent(arguments) if arguments.prompt.is_empty() => {
                Err("arguments.prompt is empty".to_string())
            }
            Action::Tool(arguments) if arguments.tool.is_empty() => {
                Err("arguments.tool is empty".to_string())
            }
            _ => Ok(()),
        }
    }
}
