// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::sync::Mutex;

use app_domains::{
    core::models::{AppContext, AppError},
    tools::traits::OutputWriter,
};

#[derive(Default)]
pub struct Memory {
    writes: Mutex<Vec<(String, String)>>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn writes(&self) -> Vec<(String, String)> {
        self.writes.lock().unwrap().clone()
    }
}

#[async_trait]
impl OutputWriter for Memory {
    async fn write(&self, _ctx: &AppContext, path: &str, content: &str) -> Result<(), AppError> {
        self.writes
            .lock()
            .unwrap()
            .push((path.to_string(), content.to_string()));
        Ok(())
    }
}
