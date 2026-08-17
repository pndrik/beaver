// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use super::traits::SubagentTool;
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{Inference, models::Conversation},
    tools::{
        Tools,
        models::{Call, Tool, ToolPermission},
    },
};

pub(super) struct SubagentToolSet {
    tools: Vec<Box<dyn SubagentTool + Send + Sync>>,
}

impl SubagentToolSet {
    pub(super) fn new(tools: Vec<Box<dyn SubagentTool + Send + Sync>>) -> Self {
        Self { tools }
    }

    pub(super) async fn list(&self, ctx: &AppContext) -> Result<Vec<Tool>, AppError> {
        let mut tools = Vec::new();
        for tool in &self.tools {
            tools.push(Tool {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                schema: tool.schema(ctx).await?,
            });
        }

        Ok(tools)
    }

    pub(super) async fn call(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        permissions: ToolPermission,
        input: &Call,
        tools: &Tools,
        inference: &Inference,
    ) -> Result<(), AppError> {
        for tool in &self.tools {
            if tool.name() == input.name {
                return tool
                    .call(ctx, conversation, permissions, input, tools, inference)
                    .await;
            }
        }

        Err(app_error!(
            Validation,
            "invalid_action",
            &format!("Invalid subagent tool: {}", input.name),
            ctx.clone()
        ))
    }
}
