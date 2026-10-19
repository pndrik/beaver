// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::core::models::{AppContext, AppError};

#[async_trait]
pub trait OutputWriter {
    async fn write(&self, ctx: &AppContext, path: &str, content: &str) -> Result<(), AppError>;
}
