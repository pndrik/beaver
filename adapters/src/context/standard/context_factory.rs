// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};

use app_domains::core::{
    models::{AppContext, AppError},
    traits::{ContextFactory, Logger},
};

use super::ContextStandard;
use crate::configuration::ConfigurationUniversal;

/// Builds a `ContextStandard` with a fresh `ConfigurationUniversal` per context. Ignores metadata.
#[derive(Debug)]
pub struct ContextFactoryStandard {
    logger: Arc<dyn Logger + Send + Sync>,
}

impl ContextFactoryStandard {
    pub fn new(logger: Arc<dyn Logger + Send + Sync>) -> Self {
        Self { logger }
    }
}

#[async_trait]
impl ContextFactory for ContextFactoryStandard {
    async fn create(
        &self,
        _ctx: &AppContext,
        trace_id: Option<String>,
        _metadata: HashMap<String, String>,
    ) -> Result<AppContext, AppError> {
        let trace_id = trace_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Ok(AppContext::new(Arc::new(ContextStandard::new(
            trace_id,
            Arc::new(ConfigurationUniversal::new()?),
            self.logger.clone(),
        ))))
    }
}
