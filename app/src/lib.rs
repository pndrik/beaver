// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;
use tracing::Instrument;

use app_adapters::{configuration::ConfigurationUniversal, context::ContextStandard};
use app_domains::core::models::{AppContext, AppError};

mod app;
pub use app::*;
pub(crate) mod resolvers;

mod domains;
pub use domains::*;

const BOOT_TRACE_ID: &str = "boot";
const CONFIGURATION_LOGGER_LEVEL: &str = "logger.level";

pub async fn bootstrap() -> Result<App, AppError> {
    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        BOOT_TRACE_ID.to_string(),
        Arc::new(ConfigurationUniversal::new()?),
    )));

    let level = ctx
        .configuration()
        .get_string(&ctx, CONFIGURATION_LOGGER_LEVEL)
        .await?;
    let logging = resolvers::logging(&ctx).await?;
    logging.init(&ctx, &level)?;

    let context_factory = resolvers::context_factory(&ctx).await?;

    let span = logging.context_span(&ctx);
    App::new(ctx, context_factory, logging)
        .instrument(span)
        .await
}
