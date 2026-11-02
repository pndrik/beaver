// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{models::Agent, traits::AgentProvider},
};

#[cfg(test)]
mod tests;

pub struct Memory {
    agents: Vec<Agent>,
}

impl Memory {
    pub fn new(agents: Vec<Agent>) -> Self {
        Self { agents }
    }
}

#[async_trait]
impl AgentProvider for Memory {
    async fn reload(&self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, ctx: &AppContext) -> Result<Vec<Agent>, AppError> {
        for agent in &self.agents {
            agent.validate(ctx)?;
        }
        Ok(self.agents.clone())
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Agent, AppError> {
        let agent = self
            .agents
            .iter()
            .find(|agent| agent.metadata.name == name)
            .ok_or_else(|| {
                app_error!(
                    NotFound,
                    "agent_not_found",
                    &format!("Agent '{name}' not found."),
                    ctx.clone()
                )
            })?;
        agent.validate(ctx)?;
        Ok(agent.clone())
    }
}
