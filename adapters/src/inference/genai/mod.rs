// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{
        models::{Conversation, Message, Model, ModelAdapter, Options, Usage},
        traits::InferenceProvider,
    },
};

mod client;
mod options;
mod request;
mod tools;
mod utils;

#[cfg(test)]
mod tests;

pub struct GenAi {
    configuration_key_endpoint: String,
    configuration_key_api_key: String,
}

impl GenAi {
    pub fn new(
        _ctx: &AppContext,
        configuration_key_endpoint: &str,
        configuration_key_api_key: &str,
    ) -> Self {
        Self {
            configuration_key_endpoint: configuration_key_endpoint.to_string(),
            configuration_key_api_key: configuration_key_api_key.to_string(),
        }
    }
}

#[async_trait]
impl InferenceProvider for GenAi {
    async fn infer(
        &self,
        ctx: &AppContext,
        options: &Options,
        conversation: &mut Conversation,
    ) -> Result<(), AppError> {
        let client = self.get_client(ctx).await?;
        let chat_options = self.get_options(ctx, conversation.agent().model(), options)?;
        let chat_request = self
            .get_chat_request(conversation)
            .with_tools(self.get_tools(ctx, conversation)?);

        let chat_response = client
            .exec_chat(
                conversation.agent().model().id(),
                chat_request,
                Some(&chat_options),
            )
            .await
            .map_err(|e| {
                app_error!(
                    Internal,
                    "inference_failed",
                    &format!("Failed to execute inference request: {}", e),
                    ctx.clone()
                )
            })?;

        let usage = &chat_response.usage;
        let details = usage.prompt_tokens_details.as_ref();
        ctx.logger
            .trace(
                ctx,
                &format!(
                    "rust-genai: Inference request completed. total_tokens: {}, cached_tokens: {}",
                    usage.total_tokens.unwrap_or(0),
                    details.and_then(|d| d.cached_tokens).unwrap_or(0),
                ),
            )
            .await;

        let usage = Usage {
            input: usage.prompt_tokens.unwrap_or(0).max(0) as u64,
            output: usage.completion_tokens.unwrap_or(0).max(0) as u64,
        };

        let content = chat_response.first_text().map(|mut text| {
            // OpenAI response does not respect stop support, so we need to manually truncate the response if stop sequences are provided
            if conversation.agent().model().adapter() == ModelAdapter::OpenAIResp {
                for stop_sequence in &options.stop_sequences {
                    if let Some(index) = text.find(stop_sequence) {
                        text = &text[..index];
                    }
                }
            }
            text.to_string()
        });

        let tool_calls = self.tool_calls_to_tool_calls(ctx, chat_response.into_tool_calls())?;
        conversation.insert_message(
            Message::agent(conversation.agent(), content, tool_calls, usage),
            None,
        );

        Ok(())
    }

    async fn supported_models(&self, ctx: &AppContext) -> Result<Vec<Model>, AppError> {
        Err(app_error!(
            Internal,
            "not_supported",
            "The GenAI inference provider does not support listing available models",
            ctx.clone()
        ))
    }

    async fn supports_model(&self, ctx: &AppContext, _model: &Model) -> Result<bool, AppError> {
        Err(app_error!(
            Internal,
            "not_supported",
            "The GenAI inference provider does not support checking if a model is supported",
            ctx.clone()
        ))
    }
}
