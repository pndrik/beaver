// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;

use super::leave;
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{Inference, models::Conversation},
    tools::{
        Tools,
        models::{Call, Schema, SchemaField, SchemaFieldType, ToolPermission},
        use_cases::subagent::{self, MAX_SUBAGENT_ITERATIONS, traits::SubagentTool},
    },
};

const DESCRIPTION: &str = "Invokes a subagent conversation. After invoking a subagent you can talk to it like you would to the user. Call 'subagent_leave' to end the conversation.";

#[derive(Debug, Deserialize)]
struct Arguments {
    name: String,
    prompt: String,
}

pub(in crate::tools::use_cases::subagent) struct Invoke;

#[async_trait]
impl SubagentTool for Invoke {
    fn name(&self) -> &'static str {
        subagent::INVOKE
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    async fn schema(&self, _ctx: &AppContext) -> Result<Schema, AppError> {
        let mut schema = Schema::new(subagent::INVOKE, DESCRIPTION);
        schema.add_property(
            "name",
            true,
            SchemaField::new(
                SchemaFieldType::String,
                "The name of the subagent to invoke.",
                None,
            ),
        );
        schema.add_property(
            "prompt",
            true,
            SchemaField::new(
                SchemaFieldType::String,
                "The prompt to send to the subagent.",
                None,
            ),
        );

        Ok(schema)
    }

    async fn call(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        permissions: ToolPermission,
        input: &Call,
        tools: &Tools,
        inference: &Inference,
    ) -> Result<(), AppError> {
        if !permissions.scopes.contains(&"_invoke".to_string()) {
            return Err(app_error!(
                Unauthorized,
                "tool_failed",
                "Can not invoke subagents without scope '_invoke'.",
                ctx.clone()
            ));
        }

        let arguments: Arguments = input.arguments_into().map_err(|e| {
            app_error!(
                Validation,
                "tool_failed",
                &format!("Invalid arguments for tool '{}': {}", subagent::INVOKE, e),
                ctx.clone()
            )
        })?;

        if !permissions.scopes.contains(&arguments.name) {
            return Err(app_error!(
                Unauthorized,
                "tool_failed",
                &format!(
                    "Can not invoke subagent '{}' without scope '{}'.",
                    arguments.name, arguments.name
                ),
                ctx.clone()
            ));
        }

        conversation.add_assistant_message(
            conversation.agent.metadata.name.clone(),
            conversation.agent.metadata.display_name.clone(),
            arguments.prompt.clone(),
        );

        let subagent_tools = conversation
            .tools
            .iter()
            .filter(|s| s.name == subagent::LIST || s.name == subagent::INVOKE)
            .cloned()
            .collect::<Vec<_>>();
        conversation.remove_tool(subagent::LIST);
        conversation.remove_tool(subagent::INVOKE);
        conversation.add_tool(leave::tool());

        let mut subagent_conversation = inference
            .new_conversation(ctx, &arguments.name, tools)
            .await?;
        subagent_conversation.add_user_message(arguments.prompt);
        conversation.add_tool_message(format!(
            "To end the conversation with the subagent '{}' you must call the '{}' tool.",
            subagent_conversation.agent.metadata.display_name,
            subagent::LEAVE
        ));

        for _ in 0..MAX_SUBAGENT_ITERATIONS {
            inference
                .infer(ctx, &mut subagent_conversation, tools)
                .await?;

            let subagent_reply = subagent_conversation.get_latest_message().unwrap();
            conversation.add_message(subagent_reply.clone());

            let left = inference
                .infer_until_leave(ctx, conversation, tools)
                .await?;
            if left {
                conversation.remove_tool(subagent::LEAVE);
                for tool in subagent_tools {
                    conversation.add_tool(tool);
                }

                conversation.add_tool_message(format!(
                    "The subagent '{}' has left the conversation.",
                    arguments.name
                ));
                break;
            }

            let latest_message = conversation.get_latest_message_content().unwrap();
            subagent_conversation.add_user_message(latest_message.clone());
        }

        Ok(())
    }
}
