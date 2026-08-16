// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::core::models::{AppContext, AppError};
use crate::tools::models::{Call, Tool, ToolPermission};

#[async_trait]
pub trait ToolsProvider {
    async fn reload(&mut self, ctx: &AppContext) -> Result<(), AppError>;
    async fn list(&self, ctx: &AppContext) -> Result<Vec<Tool>, AppError>;
    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Tool, AppError>;
    async fn execute(
        &self,
        ctx: &AppContext,
        permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError>;
}
