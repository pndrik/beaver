// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{models::Agent, traits::AgentProvider},
};

mod file;
mod parse;

const CONFIGURATION_AGENTS_DIR: &str = "agents.path";

pub struct File {}

impl File {
    pub fn new(_ctx: &AppContext) -> Self {
        Self {}
    }

    async fn get_agents_dir(&self, ctx: &AppContext) -> Result<String, AppError> {
        ctx.configuration
            .get_string(ctx, CONFIGURATION_AGENTS_DIR)
            .await
    }

    fn load_agent(&self, ctx: &AppContext, agent_dir: &str) -> Result<Agent, AppError> {
        let folder_name = agent_dir.split('/').last().unwrap_or("");
        let config_content = file::read_file(ctx, &format!("{}/config.yaml", agent_dir))?;
        let mut agent = parse::parse_agent(ctx, &config_content)?;

        if agent.metadata.name.is_empty() || agent.metadata.name != folder_name {
            return Err(app_error!(
                Internal,
                "configuration_load_failed",
                &format!(
                    "Agent name is missing or missmatching in config: {}",
                    agent_dir
                ),
                ctx.clone()
            ));
        }

        let prompt = file::read_file(ctx, &format!("{}/prompt.md", agent_dir))?;
        agent.prompt = prompt;

        Ok(agent)
    }
}

#[async_trait]
impl AgentProvider for File {
    async fn reload(&self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, ctx: &AppContext) -> Result<Vec<Agent>, AppError> {
        let agents_dir = self.get_agents_dir(ctx).await?;
        let mut valid_agents = Vec::new();
        let agents = file::list_directories(ctx, &agents_dir)?;

        for agent in agents {
            let agent = self.load_agent(ctx, &agent)?;
            valid_agents.push(agent);
        }

        Ok(valid_agents)
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Agent, AppError> {
        let agents_dir = self.get_agents_dir(ctx).await?;
        self.load_agent(ctx, &format!("{}/{}", agents_dir, name))
    }
}
