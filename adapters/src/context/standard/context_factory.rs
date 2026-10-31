// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::sync::Arc;

use app_domains::core::{
    models::{AppContext, AppError},
    traits::ContextFactory,
};

use super::ContextStandard;
use crate::configuration::ConfigurationUniversal;

#[derive(Debug, Default)]
pub struct ContextFactoryStandard;

impl ContextFactoryStandard {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ContextFactory for ContextFactoryStandard {
    async fn create(
        &self,
        _ctx: &AppContext,
        trace_id: Option<String>,
    ) -> Result<AppContext, AppError> {
        let trace_id = trace_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Ok(AppContext::new(Arc::new(ContextStandard::new(
            trace_id,
            Arc::new(ConfigurationUniversal::new()?),
        ))))
    }
}
