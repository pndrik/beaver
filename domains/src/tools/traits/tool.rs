// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::{
    core::models::{AppContext, AppError},
    tools::models::{self, Call, Schema, ToolPermission},
};

#[async_trait]
pub trait Tool {
    async fn name(&self, ctx: &AppContext) -> Result<String, AppError>;
    async fn description(&self, ctx: &AppContext) -> Result<String, AppError>;
    async fn schema(&self, ctx: &AppContext) -> Result<Schema, AppError>;
    async fn call(
        &self,
        ctx: &AppContext,
        permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError>;
    async fn as_tool(&self, ctx: &AppContext) -> Result<models::Tool, AppError> {
        Ok(models::Tool {
            name: self.name(ctx).await?,
            description: self.description(ctx).await?,
            schema: self.schema(ctx).await?,
        })
    }
}
