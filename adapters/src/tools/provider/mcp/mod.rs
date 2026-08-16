// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::collections::HashMap;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::models,
    tools::models::Call,
    tools::models::ToolPermission,
    tools::traits::ToolsProvider,
};

mod mcp_server;
use mcp_server::McpServer;

mod mcp_session;
use mcp_session::McpSession;

mod mcp_tool;
use mcp_tool::McpTool;

pub struct McpProvider {
    tools: HashMap<String, McpTool>,
    server: HashMap<String, McpServer>,
}

const MCP_SERVERS_CONFIGURATION_KEY: &str = "tools.providers.mcp.servers";

impl McpProvider {
    pub fn new(_ctx: &AppContext) -> Self {
        Self {
            tools: HashMap::new(),
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
impl ToolsProvider for McpProvider {
    async fn reload(&mut self, ctx: &AppContext) -> Result<(), AppError> {
        let servers = self.get_servers(ctx).await?;

        let mut servers_map: HashMap<String, McpServer> = HashMap::new();
        let mut tools_map: HashMap<String, McpTool> = HashMap::new();

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

            let server_tools = match session.list_tools(ctx).await {
                Ok(tools) => tools,
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

            for tool in server_tools {
                let tool_name = format!("mcp_{}_{}", server.name, tool.name);
                tools_map.insert(tool_name, McpTool::new(server.name.clone(), tool));
            }
        }

        self.server = servers_map;
        self.tools = tools_map;

        Ok(())
    }

    async fn list(&self, _ctx: &AppContext) -> Result<Vec<models::Tool>, AppError> {
        self.tools
            .iter()
            .map(|(name, tool)| {
                Ok(models::Tool {
                    name: name.clone(),
                    description: tool.tool.description.clone(),
                    schema: tool.tool.schema.clone(),
                })
            })
            .collect::<Result<Vec<app_domains::tools::models::Tool>, AppError>>()
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<models::Tool, AppError> {
        let tool = self.tools.get(name).ok_or_else(|| {
            app_error!(
                NotFound,
                "tool_not_found",
                &format!("Tool with name '{}' not found", name),
                ctx.clone()
            )
        })?;

        Ok(models::Tool {
            name: name.to_string(),
            description: tool.tool.description.clone(),
            schema: tool.tool.schema.clone(),
        })
    }

    async fn execute(
        &self,
        ctx: &AppContext,
        _permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        let tool = self.tools.get(&input.name).ok_or_else(|| {
            app_error!(
                NotFound,
                "tool_not_found",
                &format!("Tool with name '{}' not found", &input.name),
                ctx.clone()
            )
        })?;

        let Some(server) = self.server.get(&tool.server) else {
            return Err(app_error!(
                NotFound,
                "server_not_found",
                &format!(
                    "MCP server with name '{}' not found for tool '{}'",
                    tool.server, &input.name
                ),
                ctx.clone()
            ));
        };

        let mut input = input.clone();
        input.name = tool.tool.name.clone();

        let mut session = McpSession::new(server.clone());
        session.start_session(ctx).await?;
        let result = session.call_tool(ctx, &input).await?;
        session.end_session(ctx).await?;

        Ok(result)
    }
}
