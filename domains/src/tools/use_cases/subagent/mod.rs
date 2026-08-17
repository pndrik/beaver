// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    core::models::{AppContext, AppError},
    inference::{Inference, models::Conversation},
    tools::{
        Tools,
        models::{Call, Tool, ToolPermission},
    },
};

pub(crate) const LIST: &str = "subagent_list";
pub(crate) const INVOKE: &str = "subagent_invoke";
pub(crate) const LEAVE: &str = "subagent_leave";
const MAX_SUBAGENT_ITERATIONS: usize = 50;

mod traits;

mod tool_set;
use tool_set::SubagentToolSet;

mod tools;

fn tool_set() -> SubagentToolSet {
    SubagentToolSet::new(vec![Box::new(tools::List), Box::new(tools::Invoke)])
}

impl Tools {
    pub async fn call_subagent_tool(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        permissions: ToolPermission,
        input: &Call,
        inference: &Inference,
    ) -> Result<(), AppError> {
        tool_set()
            .call(ctx, conversation, permissions, input, self, inference)
            .await
    }
}

pub(crate) async fn tools(ctx: &AppContext) -> Result<Vec<Tool>, AppError> {
    tool_set().list(ctx).await
}
