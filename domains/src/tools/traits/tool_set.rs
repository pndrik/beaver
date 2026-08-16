// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::{
    core::models::{AppContext, AppError},
    tools::models::{Call, Tool, ToolPermission},
};

#[async_trait]
pub trait ToolSet {
    async fn name(&self, ctx: &AppContext) -> Result<String, AppError>;
    async fn description(&self, ctx: &AppContext) -> Result<String, AppError>;
    async fn list(&self, ctx: &AppContext) -> Result<Vec<Tool>, AppError>;
    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Tool, AppError>;
    async fn call(
        &self,
        ctx: &AppContext,
        permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError>;
}
