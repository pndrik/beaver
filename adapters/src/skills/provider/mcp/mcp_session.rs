// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use base64::{Engine, engine::general_purpose::STANDARD};
use http::{HeaderName, HeaderValue};
use rmcp::{
    RoleClient, ServiceExt,
    model::{
        CallToolRequestParams, ClientCapabilities, ClientInfo, ContentBlock, Implementation,
        InitializeRequestParams,
    },
    service::RunningService,
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use std::{collections::HashMap, str::FromStr, sync::Arc};

use super::McpServer;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    skills::models::{Call, Schema, Skill},
};

pub(super) struct McpSession {
    pub(super) server: McpServer,

    client: Option<RunningService<RoleClient, InitializeRequestParams>>,
}

impl McpSession {
    pub fn new(server: McpServer) -> Self {
        Self {
            server,

            client: None,
        }
    }

    fn get_auth_headers(
        &self,
        ctx: &AppContext,
    ) -> Result<HashMap<HeaderName, HeaderValue>, AppError> {
        let Some(credentials) = &self.server.credentials else {
            return Ok(HashMap::new());
        };

        if credentials.basic.is_some() && credentials.bearer.is_some() {
            return Err(app_error!(
                Validation,
                "configuration_load_failed",
                "MCP server configuration cannot have both basic and bearer credentials set",
                ctx.clone()
            ));
        }

        let mut headers: HashMap<HeaderName, HeaderValue> = HashMap::new();

        if let Some(basic) = &credentials.basic {
            let auth_value = format!(
                "Basic {}",
                STANDARD.encode(format!("{}:{}", basic.username, basic.password))
            );
            headers.insert(
                HeaderName::from_static("authorization"),
                HeaderValue::from_str(&auth_value).map_err(|e| {
                    app_error!(
                        Internal,
                        "configuration_load_failed",
                        format!("Failed to create basic auth header: {}", e),
                        ctx.clone()
                    )
                })?,
            );
        }

        if let Some(bearer) = &credentials.bearer {
            let auth_value = format!("Bearer {}", bearer);
            headers.insert(
                HeaderName::from_static("authorization"),
                HeaderValue::from_str(&auth_value).map_err(|e| {
                    app_error!(
                        Internal,
                        "configuration_load_failed",
                        format!("Failed to create bearer auth header: {}", e),
                        ctx.clone()
                    )
                })?,
            );
        }

        if let Some(custom_headers) = &credentials.headers {
            for (key, value) in custom_headers {
                headers.insert(
                    HeaderName::from_str(&key.to_lowercase()).map_err(|e| {
                        app_error!(
                            Internal,
                            "configuration_load_failed",
                            format!("Failed to create custom header name '{}': {}", key, e),
                            ctx.clone()
                        )
                    })?,
                    HeaderValue::from_str(value).map_err(|e| {
                        app_error!(
                            Internal,
                            "configuration_load_failed",
                            format!(
                                "Failed to create custom header value for header '{}' {}",
                                key, e
                            ),
                            ctx.clone()
                        )
                    })?,
                );
            }
        }

        Ok(headers)
    }

    pub(super) async fn start_session(&mut self, ctx: &AppContext) -> Result<(), AppError> {
        let headers = self.get_auth_headers(ctx)?;
        let transport_config =
            StreamableHttpClientTransportConfig::with_uri(self.server.url.clone())
                .custom_headers(headers);
        let transport = StreamableHttpClientTransport::from_config(transport_config);
        let client_info = ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("beaver", "0.0.1"),
        );

        self.client = Some(client_info.serve(transport).await.map_err(|e| {
            app_error!(
                Internal,
                "configuration_load_failed",
                format!("Failed to start MCP client: {}", e),
                ctx.clone()
            )
        })?);

        Ok(())
    }

    pub(super) async fn end_session(&mut self, ctx: &AppContext) -> Result<(), AppError> {
        let Some(client) = self.client.take() else {
            return Err(app_error!(
                Internal,
                "internal_error",
                "MCP client is not initialized. Call start_session() first.",
                ctx.clone()
            ));
        };

        client.cancel().await.map_err(|e| {
            app_error!(
                Internal,
                "internal_error",
                format!("Failed to stop MCP client: {}", e),
                ctx.clone()
            )
        })?;

        Ok(())
    }

    pub(super) async fn list_tools(&self, ctx: &AppContext) -> Result<Vec<Skill>, AppError> {
        let Some(client) = &self.client else {
            return Err(app_error!(
                Internal,
                "internal_error",
                "MCP client is not initialized. Call start_session() first.",
                ctx.clone()
            ));
        };

        let tools = client.list_tools(Default::default()).await.map_err(|e| {
            app_error!(
                Internal,
                "internal_error",
                format!("Failed to list tools from MCP server: {}", e),
                ctx.clone()
            )
        })?;

        Ok(tools
            .tools
            .into_iter()
            .filter_map(|tool| {
                let description = tool.description?;
                let parameters = match Schema::from_json_input_schema(
                    ctx,
                    Arc::unwrap_or_clone(tool.input_schema.clone()),
                ) {
                    Ok(schema) => schema,
                    Err(e) => {
                        println!(
                            "Failed to parse input schema for tool '{}': {}",
                            tool.name, e.internal_message
                        );
                        return None;
                    }
                };

                Some(Skill {
                    name: tool.name.to_string(),
                    description: description.to_string(),
                    parameters: parameters,
                })
            })
            .collect())
    }

    pub(super) async fn call_tool(
        &self,
        ctx: &AppContext,
        input: &Call,
    ) -> Result<String, AppError> {
        let Some(client) = &self.client else {
            return Err(app_error!(
                Internal,
                "internal_error",
                "MCP client is not initialized. Call start_session() first.",
                ctx.clone()
            ));
        };

        let tool_result = client
            .call_tool(
                CallToolRequestParams::new(input.name.clone())
                    .with_arguments(input.arguments_as_json_map()),
            )
            .await
            .map_err(|e| {
                app_error!(
                    Internal,
                    "internal_error",
                    format!("Failed to call tool '{}' on MCP server: {}", input.name, e),
                    ctx.clone()
                )
            })?;

        if let Some(result_type) = tool_result.result_type {
            if result_type.as_str() != "success" && result_type.as_str() != "text" {
                return Err(app_error!(
                    Internal,
                    "tool_call_failed",
                    format!(
                        "Tool '{}' call failed with result type '{}'",
                        input.name, result_type
                    ),
                    ctx.clone()
                ));
            }
        }

        let mut all_text: String = tool_result
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        if let Some(obj) = &tool_result.structured_content {
            let json = serde_json::to_string(obj).unwrap_or_default();
            if !all_text.is_empty() {
                all_text.push('\n');
            }
            all_text.push_str(&json);
        }

        Ok(all_text)
    }
}
