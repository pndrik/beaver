// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use genai::chat::{Tool, ToolCall};

use super::{GenAi, utils::convert_cache_control};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::models::Conversation,
    tools::models::Call,
};

impl GenAi {
    pub(super) fn get_tools(
        &self,
        ctx: &AppContext,
        conversation: &Conversation,
    ) -> Result<Vec<Tool>, AppError> {
        let caching = &conversation.agent.inference.caching;
        let cache_control =
            convert_cache_control(caching.system.as_ref().unwrap_or(&caching.default));

        let mut tools_sorted = conversation.tools.clone();
        tools_sorted.sort_by(|a, b| a.name.cmp(&b.name));

        let mut tools = tools_sorted
            .iter()
            .map(|tool| {
                let schema = tool.schema.to_json_value(ctx)?;
                Ok(Tool::new(&tool.name)
                    .with_description(&tool.description)
                    .with_schema(schema))
            })
            .collect::<Result<Vec<Tool>, AppError>>()?;

        if let Some(cache_level) = cache_control
            && let Some(last_tool) = tools.last_mut()
        {
            last_tool.cache_control = Some(cache_level);
        }

        Ok(tools)
    }

    pub(super) fn tool_calls_to_tool_calls(
        &self,
        ctx: &AppContext,
        tool_calls: Vec<ToolCall>,
    ) -> Result<Vec<Call>, AppError> {
        let mut calls: Vec<Call> = vec![];
        for tool_call in &tool_calls {
            calls.push(Call {
                name: tool_call.fn_name.clone(),
                arguments: serde_json::from_value(tool_call.fn_arguments.clone()).map_err(|e| {
                    app_error!(
                        Internal,
                        "inference_failed",
                        &format!("Failed to parse tool call arguments: {}", e),
                        ctx.clone()
                    )
                })?,
            });
        }

        Ok(calls)
    }
}
