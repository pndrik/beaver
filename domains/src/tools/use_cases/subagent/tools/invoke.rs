// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;

use super::leave;
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{
        Inference,
        models::{Conversation, Message, Usage},
    },
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

        conversation.insert_message(
            Message::agent(
                conversation.agent(),
                Some(arguments.prompt.clone()),
                vec![],
                conversation.usage().unwrap_or_default(),
            ),
            None,
        );

        let subagent_tools = conversation
            .tools()
            .iter()
            .filter(|s| s.name == subagent::LIST || s.name == subagent::INVOKE)
            .cloned()
            .collect::<Vec<_>>();
        conversation.remove_tool(subagent::LIST);
        conversation.remove_tool(subagent::INVOKE);
        conversation.add_tool(leave::tool());

        let chroot = conversation.chroot().to_string();
        let mut subagent_conversation = inference
            .new_conversation(ctx, &arguments.name, tools, &chroot)
            .await?;
        subagent_conversation.insert_message(
            Message::user(arguments.prompt, subagent_conversation.agent()),
            None,
        );
        conversation.insert_message(
            Message::tool(
                format!(
                    "To end the conversation with the subagent '{}' you must call the '{}' tool.",
                    subagent_conversation.agent().metadata.display_name,
                    subagent::LEAVE
                ),
                conversation.agent(),
            ),
            None,
        );

        for _ in 0..MAX_SUBAGENT_ITERATIONS {
            inference
                .infer(ctx, &mut subagent_conversation, tools)
                .await?;

            let subagent_reply = match subagent_conversation.last() {
                Some(message) => message.clone(),
                None => Message::agent(
                    subagent_conversation.agent(),
                    Some("Subagent did not return a response.".to_string()),
                    vec![],
                    Usage::default(),
                ),
            };
            conversation.insert_message(subagent_reply, None);

            let left = inference
                .infer_until_leave(ctx, conversation, tools)
                .await?;
            if left {
                conversation.remove_tool(subagent::LEAVE);
                for tool in subagent_tools {
                    conversation.add_tool(tool);
                }

                conversation.insert_message(
                    Message::tool(
                        format!(
                            "The subagent '{}' has left the conversation.",
                            arguments.name
                        ),
                        conversation.agent(),
                    ),
                    None,
                );
                break;
            }

            let latest_message = conversation
                .last()
                .and_then(|message| message.as_agent())
                .and_then(|message| message.content.clone())
                .unwrap_or("The parent agent did not provide a message".to_string());

            subagent_conversation.insert_message(
                Message::user(latest_message.clone(), subagent_conversation.agent()),
                None,
            );
        }

        Ok(())
    }
}
