// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::{
        models::{Call, Tool, ToolPermission},
        traits::{Tool as ToolTrait, ToolSet},
    },
};

pub struct EmbeddedToolSet {
    name: String,
    description: String,
    tools: Vec<Box<dyn ToolTrait + Send + Sync>>,
}

impl EmbeddedToolSet {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        tools: Vec<Box<dyn ToolTrait + Send + Sync>>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            tools,
        }
    }
}

#[async_trait]
impl ToolSet for EmbeddedToolSet {
    async fn name(&self, _ctx: &AppContext) -> Result<String, AppError> {
        Ok(self.name.clone())
    }

    async fn description(&self, _ctx: &AppContext) -> Result<String, AppError> {
        Ok(self.description.clone())
    }

    async fn list(&self, ctx: &AppContext) -> Result<Vec<Tool>, AppError> {
        let mut tools = Vec::with_capacity(self.tools.len());
        for tool in &self.tools {
            tools.push(tool.as_tool(ctx).await?);
        }
        Ok(tools)
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Tool, AppError> {
        for tool in &self.tools {
            if tool.name(ctx).await? == name {
                return tool.as_tool(ctx).await;
            }
        }

        Err(app_error!(
            NotFound,
            "tool_not_found",
            &format!("Tool with name '{}' not found", name),
            ctx.clone()
        ))
    }

    async fn call(
        &self,
        ctx: &AppContext,
        permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        for tool in &self.tools {
            if tool.name(ctx).await? == input.name {
                return tool.call(ctx, permissions, input).await;
            }
        }

        Err(app_error!(
            Validation,
            "tool_not_found",
            &format!("Invalid tool: {}", input.name),
            ctx.clone()
        ))
    }
}
