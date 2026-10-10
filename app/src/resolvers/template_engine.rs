// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::template_engine::MiniJinjaTemplateEngine;
use app_domains::{
    core::models::{AppContext, AppError},
    core::traits::TemplateEngine,
};

pub async fn template_engine(
    ctx: &AppContext,
) -> Result<Arc<dyn TemplateEngine + Send + Sync>, AppError> {
    Ok(Arc::new(MiniJinjaTemplateEngine::new(ctx)))
}
