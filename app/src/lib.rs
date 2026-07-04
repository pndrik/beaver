// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::configuration::ConfigurationUniversal;
use app_domains::core::models::{AppContext, AppError};

mod app;
pub use app::*;
pub(crate) mod resolvers;

mod domains;
pub use domains::*;

pub async fn bootstrap(ctx: &AppContext) -> Result<App, AppError> {
    let app = App::new(ctx).await?;

    Ok(app)
}

pub async fn context(id: String) -> Result<AppContext, AppError> {
    let configuration = Arc::new(ConfigurationUniversal::new()?);
    let ctx = AppContext::new(id.clone(), "/tmp".to_string(), configuration.clone())?;

    let chroot = ctx.configuration.get_string(&ctx, "context.chroot").await?;
    let ctx = AppContext::new(id, chroot, configuration.clone())?;

    Ok(ctx)
}
