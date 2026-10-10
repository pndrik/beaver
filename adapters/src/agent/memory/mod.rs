// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{models::Agent, traits::AgentProvider},
};

/// Serves a fixed list of agents held in memory.
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

    async fn list(&self, _ctx: &AppContext) -> Result<Vec<Agent>, AppError> {
        Ok(self.agents.clone())
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Agent, AppError> {
        self.agents
            .iter()
            .find(|agent| agent.metadata.name == name)
            .cloned()
            .ok_or_else(|| {
                app_error!(
                    NotFound,
                    "agent_not_found",
                    &format!("Agent '{}' not found.", name),
                    ctx.clone()
                )
            })
    }
}
