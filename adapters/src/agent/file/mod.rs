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

    fn load_agent(&self, ctx: &AppContext, agent_path: &str) -> Result<Agent, AppError> {
        let mut agent: Agent;
        if agent_path.ends_with(".yaml") {
            let config_content = file::read_file(ctx, agent_path)?;
            agent = parse::parse_agent(ctx, &config_content)?;
        } else {
            let config_path = format!("{}/config.yaml", agent_path);
            let config_content = file::read_file(ctx, &config_path)?;
            agent = parse::parse_agent(ctx, &config_content)?;

            if agent.prompt.is_empty() {
                let prompt_path = format!("{}/prompt.md", agent_path);
                agent.prompt = file::read_file(ctx, &prompt_path)?;
            }
        }

        let filesystem_name = agent_path
            .split('/')
            .last()
            .ok_or_else(|| {
                app_error!(
                    Internal,
                    "configuration_load_failed",
                    &format!("Failed to extract agent name from path: {}", agent_path),
                    ctx.clone()
                )
            })?
            .trim_end_matches(".yaml")
            .to_string();

        if agent.metadata.name.is_empty() || agent.metadata.name != filesystem_name {
            return Err(app_error!(
                Internal,
                "configuration_load_failed",
                &format!(
                    "Agent name is missing or missmatching in config: {}",
                    agent_path
                ),
                ctx.clone()
            ));
        }

        if agent.prompt.is_empty() {
            return Err(app_error!(
                Internal,
                "configuration_load_failed",
                &format!("Agent prompt is missing in config: {}", agent_path),
                ctx.clone()
            ));
        }

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

        let yaml_path = format!("{}/{}.yaml", agents_dir, name);
        if file::file_exists(&yaml_path) {
            return self.load_agent(ctx, &yaml_path);
        }

        self.load_agent(ctx, &format!("{}/{}", agents_dir, name))
    }
}
