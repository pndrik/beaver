// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    core::models::{AppContext, AppError},
    inference::{
        Inference,
        models::{Conversation, Message},
    },
    tools::{Tools, models::Tool},
};

const PROMPT: &str = include_str!("prompt.md");

impl Inference {
    pub async fn new_conversation(
        &self,
        ctx: &AppContext,
        agent_name: &str,
        tools: &Tools,
        chroot: &str,
    ) -> Result<Conversation, AppError> {
        let agent = self.agent_provider.get(ctx, agent_name).await?;

        let tools_filtered = tools
            .list_all(ctx)
            .await?
            .iter()
            .filter(|s| agent.tools.has_tool_permission(&s.name))
            .cloned()
            .collect::<Vec<Tool>>();

        let system_prompt = Message::system(
            PROMPT.to_string() + "\n\n---\n# Harness\n" + &agent.prompt,
            &agent,
        );
        Conversation::new(ctx, system_prompt, agent, tools_filtered, chroot)
    }
}
