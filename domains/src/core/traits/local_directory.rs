// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::path::Path;

use crate::core::models::{AppContext, AppError};

#[async_trait]
pub trait LocalDirectory {
    fn path(&self) -> &Path;

    async fn sync(self: Box<Self>, ctx: &AppContext) -> Result<(), AppError>;
}
