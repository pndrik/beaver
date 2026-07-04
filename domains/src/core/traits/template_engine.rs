// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde_json::Value;

use crate::core::models::{AppContext, AppError};

#[async_trait]
pub trait TemplateEngine {
    async fn render(
        &self,
        ctx: &AppContext,
        template: &str,
        values: &Value,
    ) -> Result<String, AppError>;
}
