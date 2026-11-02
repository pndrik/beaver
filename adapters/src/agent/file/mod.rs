// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{models::Agent, traits::AgentProvider},
};

mod io;
mod parse;

#[cfg(test)]
mod tests;

const CONFIGURATION_AGENTS_DIR: &str = "agents.path";

pub struct File {}

impl File {
    pub fn new(_ctx: &AppContext) -> Self {
        Self {}
    }

    async fn get_agents_dir(&self, ctx: &AppContext) -> Result<String, AppError> {
        let path = ctx
            .configuration()
            .get_string(ctx, CONFIGURATION_AGENTS_DIR)
            .await?;
        Ok(path.trim_end_matches('/').to_string())
    }

    fn list_agent_names(ctx: &AppContext, agents_dir: &str) -> Result<Vec<String>, AppError> {
        io::list_yaml_files(ctx, agents_dir)?
            .iter()
            .map(|path| {
                path.split('/')
                    .next_back()
                    .and_then(|name| name.strip_suffix(".yaml"))
                    .map(std::string::ToString::to_string)
                    .ok_or_else(|| {
                        app_error!(
                            Internal,
                            "configuration_load_failed",
                            &format!("Invalid agent file name: {path}"),
                            ctx.clone()
                        )
                    })
            })
            .collect()
    }

    fn load_agent(ctx: &AppContext, agents_dir: &str, name: &str) -> Result<Agent, AppError> {
        let path = format!("{agents_dir}/{name}.yaml");
        let agent = parse::parse_agent(ctx, &path, &io::read_file(ctx, &path)?)?;

        if agent.metadata.name != name {
            return Err(app_error!(
                Validation,
                "agent_invalid",
                &format!(
                    "Agent name mismatch: expected '{}', found '{}'",
                    name, agent.metadata.name
                ),
                ctx.clone()
            ));
        }

        agent.validate(ctx)?;

        Ok(agent)
    }
}

fn is_plain_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\']) && !name.starts_with('.')
}

#[async_trait]
impl AgentProvider for File {
    async fn reload(&self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, ctx: &AppContext) -> Result<Vec<Agent>, AppError> {
        let agents_dir = self.get_agents_dir(ctx).await?;

        File::list_agent_names(ctx, &agents_dir)?
            .iter()
            .map(|name| File::load_agent(ctx, &agents_dir, name))
            .collect()
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Agent, AppError> {
        let agents_dir = self.get_agents_dir(ctx).await?;

        if !is_plain_name(name)
            || !io::file_exists(&format!("{agents_dir}/{name}.yaml"))
            || !File::list_agent_names(ctx, &agents_dir)?
                .iter()
                .any(|n| n == name)
        {
            return Err(app_error!(
                NotFound,
                "agent_not_found",
                &format!("Agent '{name}' not found."),
                ctx.clone()
            ));
        }

        File::load_agent(ctx, &agents_dir, name)
    }
}
