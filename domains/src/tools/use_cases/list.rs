// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    core::models::{AppContext, AppError},
    tools::{
        Tools,
        models::{Tool, ToolsConfiguration},
        use_cases::{redirect, subagent},
    },
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

    pub async fn list_permitted(
        &self,
        ctx: &AppContext,
        configuration: &ToolsConfiguration,
    ) -> Result<Vec<Tool>, AppError> {
        let mut tools_found = Vec::new();
        for provider in &self.tools_providers {
            tools_found.extend(
                provider
                    .list(ctx)
                    .await?
                    .into_iter()
                    .filter(|tool| configuration.has_tool_permission(&tool.name))
                    .map(|tool| match configuration.redirect_output {
                        true => redirect::with_redirect(tool),
                        false => tool,
                    }),
            );
        }
        tools_found.extend(
            subagent::tools(ctx)
                .await?
                .into_iter()
                .filter(|tool| configuration.has_tool_permission(&tool.name)),
        );

        Ok(tools_found)
    }
}
