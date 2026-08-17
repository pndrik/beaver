// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::tools::{
    models::{Schema, Tool},
    use_cases::subagent,
};

const DESCRIPTION: &str = "Leave the subagent conversation.";

// Not a `SubagentTool` - this is a sentinel tool spliced into a conversation's
// tool list only while a subagent session is active. It is never dispatched
// through the registry; `Inference::infer_until_leave` detects it by name.
pub(in crate::tools::use_cases::subagent) fn tool() -> Tool {
    Tool {
        name: subagent::LEAVE.to_string(),
        description: DESCRIPTION.to_string(),
        schema: Schema::new(subagent::LEAVE, DESCRIPTION),
    }
}
