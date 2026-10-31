// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::{collections::HashMap, fmt::Debug};

use crate::core::models::{AppContext, AppError};

#[async_trait]
pub trait ContextFactory: Debug {
    async fn create(
        &self,
        ctx: &AppContext,
        trace_id: Option<String>,
        metadata: HashMap<String, String>,
    ) -> Result<AppContext, AppError>;
}
