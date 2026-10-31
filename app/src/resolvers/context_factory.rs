// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::context::ContextFactoryStandard;
use app_domains::core::{
    models::{AppContext, AppError},
    traits::{ContextFactory, Logger},
};

pub async fn context_factory(
    _ctx: &AppContext,
    logger: Arc<dyn Logger + Send + Sync>,
) -> Result<Arc<dyn ContextFactory + Send + Sync>, AppError> {
    Ok(Arc::new(ContextFactoryStandard::new(logger)))
}
