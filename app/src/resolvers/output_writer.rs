// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::tools::output_writer::File;
use app_domains::{
    core::models::{AppContext, AppError},
    tools::traits::OutputWriter,
};

pub async fn output_writer(
    _ctx: &AppContext,
) -> Result<Arc<dyn OutputWriter + Send + Sync>, AppError> {
    Ok(Arc::new(File::new()))
}
