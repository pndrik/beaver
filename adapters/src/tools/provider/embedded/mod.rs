// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::sync::Arc;

mod models;
pub use models::EmbeddedToolSet;

use crate::tools::embedded::{File, Git};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::{
        models::{Call, Tool, ToolPermission},
        traits::{ToolSet, ToolsProvider},
    },
};

pub struct EmbeddedProvider {
    tool_sets: Vec<Arc<dyn ToolSet + Send + Sync>>,
}

impl EmbeddedProvider {
    pub fn new(_ctx: &AppContext) -> Self {
        Self {
            tool_sets: vec![Arc::new(File::new()), Arc::new(Git::new())],
        }
    }
}

#[async_trait]
impl ToolsProvider for EmbeddedProvider {
    async fn reload(&mut self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(
        &self,
        ctx: &AppContext,
    ) -> Result<Vec<app_domains::tools::models::Tool>, AppError> {
        let mut tools = Vec::new();
        for tool_set in &self.tool_sets {
            tools.extend(tool_set.list(ctx).await?);
        }
        Ok(tools)
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Tool, AppError> {
        for tool_set in &self.tool_sets {
            match tool_set.get(ctx, name).await {
                Ok(tool) => return Ok(tool),
                Err(_) => continue,
            }
        }

        Err(app_error!(
            NotFound,
            "tool_not_found",
            &format!("Tool with name '{}' not found", name),
            ctx.clone()
        ))
    }

    async fn execute(
        &self,
        ctx: &AppContext,
        permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        for tool_set in &self.tool_sets {
            match tool_set.get(ctx, &input.name).await {
                Err(_) => continue,
                Ok(_) => {
                    return tool_set.call(ctx, permissions, input).await;
                }
            }
        }

        Err(app_error!(
            NotFound,
            "tool_not_found",
            &format!("Tool with name '{}' not found", input.name),
            ctx.clone()
        ))
    }
}
