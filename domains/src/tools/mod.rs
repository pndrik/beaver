// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use crate::core::models::AppError;

pub mod models;
pub mod traits;
pub mod use_cases;

pub struct Tools {
    pub tools_providers: Vec<Arc<dyn traits::ToolsProvider + Send + Sync>>,
}

impl Tools {
    pub fn new(
        tools_providers: Vec<Arc<dyn traits::ToolsProvider + Send + Sync>>,
    ) -> Result<Self, AppError> {
        Ok(Self { tools_providers })
    }
}
