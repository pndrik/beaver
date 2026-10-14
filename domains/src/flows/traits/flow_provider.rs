// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::core::models::{AppContext, AppError};
use crate::flows::models::Definition;

#[async_trait]
pub trait FlowProvider {
    async fn reload(&self, ctx: &AppContext) -> Result<(), AppError>;
    async fn list(&self, ctx: &AppContext) -> Result<Vec<Definition>, AppError>;
    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Definition, AppError>;
    /// The flow whose webhook trigger is `name`, if `token` is valid for it.
    /// `None` for an unknown name and for a wrong token alike. How tokens are
    /// stored and compared is up to the provider.
    async fn get_by_webhook(
        &self,
        ctx: &AppContext,
        name: &str,
        token: &str,
    ) -> Result<Option<Definition>, AppError>;
}
