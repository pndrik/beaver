// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::core::models::{AppContext, AppError};
use crate::inference::models::Webhook;

#[async_trait]
pub trait WebhookProvider {
    async fn reload(&self, ctx: &AppContext) -> Result<(), AppError>;
    async fn list(&self, ctx: &AppContext) -> Result<Vec<Webhook>, AppError>;
    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Webhook, AppError>;
}
