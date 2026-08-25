// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{
        Inference,
        models::{Conversation, Model, Options},
        traits::InferenceProvider,
    },
    tools::{Tools, use_cases::subagent},
};

const MAX_INFERENCE_TOOL_ITERATIONS: usize = 100;

async fn find_inference_provider(
    ctx: &AppContext,
    providers: &[Arc<dyn InferenceProvider + Send + Sync>],
    model: &Model,
) -> Result<Arc<dyn InferenceProvider + Send + Sync>, AppError> {
    for provider in providers {
        if provider.supports_model(ctx, model).await? {
            return Ok(provider.clone());
        }
    }

    Err(app_error!(
        Internal,
        "inference_provider_not_found",
        &format!("No inference provider found for model: {}", model.id()),
        ctx.clone()
    ))
}

impl Inference {
    fn poll<'a>(
        &'a self,
        ctx: &'a AppContext,
        conversation: &'a mut Conversation,
        tools: &'a Tools,
        leave_tool: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, AppError>> + Send + 'a>> {
        Box::pin(async move {
            let inference_provider = find_inference_provider(
                ctx,
                &self.inference_providers,
                &conversation.agent.model(),
            )
            .await?;
            let options = Options::default();

            for _ in 0..MAX_INFERENCE_TOOL_ITERATIONS {
                let tool_calls = inference_provider
                    .infer(ctx, &options, conversation)
                    .await?;

                let Some(latest_message) = conversation.get_latest_message() else {
                    break;
                };

                if tool_calls.is_empty() {
                    if !latest_message.is_assistant() {
                        return Err(app_error!(
                            Internal,
                            "invalid_response_format",
                            "Latest message is not from the assistant and there were no tool calls.",
                            ctx.clone()
                        ));
                    }

                    return Ok(false);
                }

                if let Some(sentinel) = leave_tool {
                    if tool_calls.iter().any(|call| call.name == sentinel) {
                        return Ok(true);
                    }
                }

                tools.call_many(ctx, conversation, self, tool_calls).await?;
            }

            ctx.logger
                .warn(
                    ctx,
                    &format!(
                        "Maximum inference tool iterations ({}) exceeded",
                        MAX_INFERENCE_TOOL_ITERATIONS
                    ),
                )
                .await;
            conversation.add_tool_message(format!(
                "Error: \nMaximum number of tool iterations ({}) exceeded",
                MAX_INFERENCE_TOOL_ITERATIONS
            ));

            Ok(false)
        })
    }

    pub async fn infer(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        tools: &Tools,
    ) -> Result<(), AppError> {
        self.poll(ctx, conversation, tools, None).await?;
        Ok(())
    }

    pub async fn infer_until_leave(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        tools: &Tools,
    ) -> Result<bool, AppError> {
        self.poll(ctx, conversation, tools, Some(subagent::LEAVE))
            .await
    }
}
