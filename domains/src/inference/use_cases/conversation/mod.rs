// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    core::models::{AppContext, AppError},
    inference::{Inference, models::Conversation},
    skills::{Skills, models::Skill, use_cases::subagent},
};

const PROMPT: &str = include_str!("prompt.md");

impl Inference {
    pub async fn new_conversation(
        &self,
        ctx: &AppContext,
        agent_name: &str,
        skills: &Skills,
    ) -> Result<Conversation, AppError> {
        let agent = self.agent_provider.get(ctx, agent_name).await?;

        let mut skills_found = Vec::new();
        for provider in &skills.skills_providers {
            skills_found.extend(provider.list(ctx).await?);
        }

        skills_found.push(Skill {
            name: subagent::NAME.to_string(),
            description: subagent::DESCRIPTION.to_string(),
            parameters: subagent::schema(ctx).await?,
        });

        let skills_filtered = skills_found
            .iter()
            .filter(|s| {
                agent.permissions.skills.iter().any(|p| p.name == s.name) || s.name == "subagent"
            })
            .cloned()
            .collect::<Vec<Skill>>();

        let conversation = Conversation::new(PROMPT.to_string(), agent, skills_filtered);

        Ok(conversation)
    }
}
