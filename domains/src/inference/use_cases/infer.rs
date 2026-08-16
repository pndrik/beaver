// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{
        Inference,
        models::{Conversation, Model, Options},
        traits::InferenceProvider,
    },
    tools::Tools,
};

const MAX_INFERENCE_TOOL_ITERATIONS: usize = 25;

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
    pub async fn infer(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        tools: &Tools,
    ) -> Result<(), AppError> {
        let inference_provider =
            find_inference_provider(ctx, &self.inference_providers, &conversation.agent.model())
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

                return Ok(());
            }

            if tool_calls.len() > 0 {
                tools
                    .call_many_with_subagent(ctx, conversation, &self, tool_calls)
                    .await?;
            }
        }

        Ok(())
    }

    // We need this for conversations with subagents as rust does not allow cyclic dependencies
    pub async fn infer_no_subagent(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        tools: &Tools,
    ) -> Result<bool, AppError> {
        let inference_provider =
            find_inference_provider(ctx, &self.inference_providers, &conversation.agent.model())
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

            if tool_calls.iter().any(|call| call.name == "subagent_leave") {
                return Ok(true);
            }

            if tool_calls.len() > 0 {
                tools.call_many(ctx, conversation, tool_calls).await?;
            }
        }

        Ok(false)
    }
}
