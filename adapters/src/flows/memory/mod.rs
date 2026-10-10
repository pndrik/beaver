// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{models::Flow, traits::FlowProvider},
};

/// Serves a fixed list of flows held in memory.
pub struct Memory {
    flows: Vec<Flow>,
}

impl Memory {
    pub fn new(flows: Vec<Flow>) -> Self {
        Self { flows }
    }
}

#[async_trait]
impl FlowProvider for Memory {
    async fn reload(&self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, _ctx: &AppContext) -> Result<Vec<Flow>, AppError> {
        Ok(self.flows.clone())
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Flow, AppError> {
        self.flows
            .iter()
            .find(|flow| flow.metadata.name == name)
            .cloned()
            .ok_or_else(|| {
                app_error!(
                    NotFound,
                    "flow_not_found",
                    &format!("Flow '{}' not found.", name),
                    ctx.clone()
                )
            })
    }

    async fn get_by_webhook(
        &self,
        _ctx: &AppContext,
        name: &str,
    ) -> Result<Option<Flow>, AppError> {
        Ok(self
            .flows
            .iter()
            .find(|flow| {
                flow.spec
                    .triggers
                    .webhook
                    .as_ref()
                    .is_some_and(|webhook| webhook.name == name)
            })
            .cloned())
    }
}
