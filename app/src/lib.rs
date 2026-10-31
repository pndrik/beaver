// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::{
    configuration::ConfigurationUniversal, context::ContextStandard, logger::LoggerAdapter,
};
use app_domains::core::models::{AppContext, AppError};

mod app;
pub use app::*;
pub(crate) mod resolvers;

mod domains;
pub use domains::*;

const BOOT_TRACE_ID: &str = "boot";

pub async fn bootstrap() -> Result<App, AppError> {
    let mut logger = LoggerAdapter::new();
    let configuration = Arc::new(ConfigurationUniversal::new()?);

    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        BOOT_TRACE_ID.to_string(),
        configuration.clone(),
        Arc::new(logger.clone()),
    )));
    logger.refresh_level(&ctx).await?;

    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        BOOT_TRACE_ID.to_string(),
        configuration,
        Arc::new(logger),
    )));
    let context_factory = resolvers::context_factory(&ctx, ctx.logger().clone()).await?;

    App::new(ctx, context_factory).await
}
