// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::tools::sandbox::Boa;
use app_domains::{
    core::models::{AppContext, AppError},
    tools::traits::CodeSandbox,
};

pub async fn javascript_sandbox(
    _ctx: &AppContext,
) -> Result<Arc<dyn CodeSandbox + Send + Sync>, AppError> {
    Ok(Arc::new(Boa::new()))
}
