// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::path::{Path, PathBuf};

use app_domains::core::{
    models::{AppContext, AppError},
    traits::LocalDirectory,
};

#[derive(Debug)]
pub struct DiskDirectory {
    pub(super) path: PathBuf,
}

#[async_trait]
impl LocalDirectory for DiskDirectory {
    fn path(&self) -> &Path {
        &self.path
    }

    async fn sync(self: Box<Self>, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }
}
