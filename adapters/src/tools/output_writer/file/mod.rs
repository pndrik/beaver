// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::traits::OutputWriter,
};

pub struct File;

impl File {
    pub fn new() -> Self {
        Self
    }
}

impl Default for File {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OutputWriter for File {
    async fn write(&self, ctx: &AppContext, path: &str, content: &str) -> Result<(), AppError> {
        std::fs::write(path, content).map_err(|e| {
            app_error!(
                ServiceUnavailable,
                "tool_failed",
                &format!("Failed to write output to '{}': {}", path, e),
                ctx.clone()
            )
        })
    }
}
