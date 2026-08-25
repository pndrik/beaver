// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    core::models::{AppContext, AppError},
    tools::{Tools, models::Tool, use_cases::subagent},
};

impl Tools {
    pub async fn list_all(&self, ctx: &AppContext) -> Result<Vec<Tool>, AppError> {
        let mut tools_found = Vec::new();
        for provider in &self.tools_providers {
            tools_found.extend(provider.list(ctx).await?);
        }
        tools_found.extend(subagent::tools(ctx).await?);

        Ok(tools_found)
    }
}
