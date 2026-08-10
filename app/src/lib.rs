// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::{configuration::ConfigurationUniversal, logger::LoggerAdapter};
use app_domains::core::models::{AppContext, AppError};

mod app;
pub use app::*;
pub(crate) mod resolvers;

mod domains;
pub use domains::*;

pub async fn bootstrap() -> Result<App, AppError> {
    let mut logger = LoggerAdapter::new();
    let configuration = Arc::new(ConfigurationUniversal::new()?);

    let mut ctx = AppContext::new(
        "boot".to_string(),
        "/tmp".to_string(),
        configuration.clone(),
        Arc::new(logger.clone()),
    )?;

    logger.refresh_level(&ctx).await?;
    ctx.logger = Arc::new(logger);

    let app = App::new(&ctx, ctx.logger.clone()).await?;

    Ok(app)
}
