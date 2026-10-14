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
