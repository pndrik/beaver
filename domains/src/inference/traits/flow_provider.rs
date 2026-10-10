// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::core::models::{AppContext, AppError};
use crate::inference::models::Flow;

#[async_trait]
pub trait FlowProvider {
    async fn reload(&self, ctx: &AppContext) -> Result<(), AppError>;
    async fn list(&self, ctx: &AppContext) -> Result<Vec<Flow>, AppError>;
    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Flow, AppError>;
    async fn get_by_webhook(&self, ctx: &AppContext, name: &str) -> Result<Option<Flow>, AppError>;
}
