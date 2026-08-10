// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    skills::models,
    skills::models::Call,
    skills::models::SkillPermission,
    skills::traits::{Skill, SkillsProvider},
};

mod mcp_server;
use mcp_server::McpServer;

mod mcp_session;
use mcp_session::McpSession;

mod mcp_skill;
use mcp_skill::McpSkill;

pub struct McpProvider {
    skills: HashMap<String, McpSkill>,
    server: HashMap<String, McpServer>,
}

const MCP_SERVERS_CONFIGURATION_KEY: &str = "skills.providers.mcp.servers";

impl McpProvider {
    pub fn new(_ctx: &AppContext) -> Self {
        Self {
            skills: HashMap::new(),
            server: HashMap::new(),
        }
    }

    async fn get_servers(&self, ctx: &AppContext) -> Result<Vec<McpServer>, AppError> {
        ctx.configuration
            .get_json_value(ctx, MCP_SERVERS_CONFIGURATION_KEY)
            .await?
            .as_array()
            .ok_or_else(|| {
                app_error!(
                    Validation,
                    "configuration_load_failed",
                    &format!(
                        "MCP servers configuration is not an array: {}",
                        MCP_SERVERS_CONFIGURATION_KEY
                    ),
                    ctx.clone()
                )
            })?
            .iter()
            .map(|value| {
                serde_json::from_value::<McpServer>(value.clone()).map_err(|e| {
                    app_error!(
                        Validation,
                        "configuration_load_failed",
                        &format!(
                            "Failed to deserialize MCP server configuration: {}",
                            e.to_string()
                        ),
                        ctx.clone()
                    )
                })
            })
            .collect::<Result<Vec<McpServer>, AppError>>()
    }
}

#[async_trait]
impl SkillsProvider for McpProvider {
    async fn reload(&mut self, ctx: &AppContext) -> Result<(), AppError> {
        let servers = self.get_servers(ctx).await?;

        let mut servers_map: HashMap<String, McpServer> = HashMap::new();
        let mut skills_map: HashMap<String, McpSkill> = HashMap::new();

        for server in servers {
            let mut session = McpSession::new(server.clone());
            if let Err(e) = session.start_session(ctx).await {
                ctx.logger
                    .warn(
                        ctx,
                        &format!(
                            "Failed to start session for MCP server '{}': {}",
                            server.name, e.internal_message
                        ),
                    )
                    .await;
                continue;
            }

            let server_skills = match session.list_tools(ctx).await {
                Ok(skills) => skills,
                Err(e) => {
                    ctx.logger
                        .warn(
                            ctx,
                            &format!(
                                "Failed to list tools from MCP server '{}': {}",
                                server.name, e.internal_message
                            ),
                        )
                        .await;
                    continue;
                }
            };
            session.end_session(ctx).await?;

            servers_map.insert(server.name.clone(), server.clone());

            for skill in server_skills {
                let skill_name = format!("mcp_{}_{}", server.name, skill.name);
                skills_map.insert(skill_name, McpSkill::new(server.name.clone(), skill));
            }
        }

        self.server = servers_map;
        self.skills = skills_map;

        Ok(())
    }

    async fn add_skill(
        &mut self,
        ctx: &AppContext,
        _skill: Arc<dyn Skill + Send + Sync>,
    ) -> Result<(), AppError> {
        Err(app_error!(
            Conflict,
            "denied",
            "Can not programmatically add skills to the McpProvider",
            ctx.clone()
        ))
    }

    async fn list(&self, _ctx: &AppContext) -> Result<Vec<models::Skill>, AppError> {
        self.skills
            .iter()
            .map(|(name, skill)| {
                Ok(models::Skill {
                    name: name.clone(),
                    description: skill.skill.description.clone(),
                    parameters: skill.skill.parameters.clone(),
                })
            })
            .collect::<Result<Vec<app_domains::skills::models::Skill>, AppError>>()
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<models::Skill, AppError> {
        let skill = self.skills.get(name).ok_or_else(|| {
            app_error!(
                NotFound,
                "skill_not_found",
                &format!("Skill with name '{}' not found", name),
                ctx.clone()
            )
        })?;

        Ok(models::Skill {
            name: name.to_string(),
            description: skill.skill.description.clone(),
            parameters: skill.skill.parameters.clone(),
        })
    }

    async fn execute(
        &self,
        ctx: &AppContext,
        _permissions: SkillPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        let skill = self.skills.get(&input.name).ok_or_else(|| {
            app_error!(
                NotFound,
                "skill_not_found",
                &format!("Skill with name '{}' not found", &input.name),
                ctx.clone()
            )
        })?;

        let Some(server) = self.server.get(&skill.server) else {
            return Err(app_error!(
                NotFound,
                "server_not_found",
                &format!(
                    "MCP server with name '{}' not found for skill '{}'",
                    skill.server, &input.name
                ),
                ctx.clone()
            ));
        };

        let mut input = input.clone();
        input.name = skill.skill.name.clone();

        let mut session = McpSession::new(server.clone());
        session.start_session(ctx).await?;
        let result = session.call_tool(ctx, &input).await?;
        session.end_session(ctx).await?;

        Ok(result)
    }
}
