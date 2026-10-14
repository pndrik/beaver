// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    flows::{models::Definition, traits::FlowProvider},
};

use super::token;

/// Serves a fixed list of flows held in memory.
pub struct Memory {
    flows: Vec<Definition>,
}

impl Memory {
    pub fn new(flows: Vec<Definition>) -> Self {
        Self { flows }
    }
}

#[async_trait]
impl FlowProvider for Memory {
    async fn reload(&self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, _ctx: &AppContext) -> Result<Vec<Definition>, AppError> {
        Ok(self.flows.clone())
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Definition, AppError> {
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
        token: &str,
    ) -> Result<Option<Definition>, AppError> {
        Ok(self
            .flows
            .iter()
            .find(|flow| token::has_webhook(flow, name))
            .filter(|flow| token::token_matches(flow, token))
            .cloned())
    }
}
