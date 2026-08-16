// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    core::models::{AppContext, AppError},
    inference::{Inference, models::Conversation},
    tools::{Tools, models::Tool, use_cases::subagent},
};

const PROMPT: &str = include_str!("prompt.md");

impl Inference {
    pub async fn new_conversation(
        &self,
        ctx: &AppContext,
        agent_name: &str,
        tools: &Tools,
    ) -> Result<Conversation, AppError> {
        let agent = self.agent_provider.get(ctx, agent_name).await?;

        let mut tools_found = tools.list_all(ctx).await?;

        tools_found.push(Tool {
            name: subagent::NAME.to_string(),
            description: subagent::DESCRIPTION.to_string(),
            schema: subagent::schema(ctx).await?,
        });

        let tools_filtered = tools_found
            .iter()
            .filter(|s| agent.permissions.has_tool_permission(&s.name) || s.name == "subagent")
            .cloned()
            .collect::<Vec<Tool>>();

        let conversation = Conversation::new(PROMPT.to_string(), agent, tools_filtered);

        Ok(conversation)
    }
}
