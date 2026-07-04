// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use genai::chat::{Tool, ToolCall};

use super::GenAi;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::models::Conversation,
    skills::models::Call as SkillCall,
};

impl GenAi {
    pub(super) fn get_tools(
        &self,
        ctx: &AppContext,
        conversation: &Conversation,
    ) -> Result<Vec<Tool>, AppError> {
        conversation
            .skills
            .iter()
            .map(|skill| {
                let schema = skill.parameters.to_json_value(ctx)?;

                Ok(Tool::new(&skill.name)
                    .with_description(&skill.description)
                    .with_schema(schema))
            })
            .collect::<Result<Vec<Tool>, AppError>>()
    }

    pub(super) fn tool_calls_to_skill_calls(
        &self,
        ctx: &AppContext,
        tool_calls: Vec<ToolCall>,
    ) -> Result<Vec<SkillCall>, AppError> {
        let mut skill_calls: Vec<SkillCall> = vec![];
        for tool_call in &tool_calls {
            skill_calls.push(SkillCall {
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

        Ok(skill_calls)
    }
}
