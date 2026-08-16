// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{
        models::{Conversation, Model, ModelAdapter, Options},
        traits::InferenceProvider,
    },
    tools::models::Call as SkillCall,
};

mod client;
mod options;
mod request;
mod tools;

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
    ) -> Result<Vec<SkillCall>, AppError> {
        let client = self.get_client(ctx).await?;
        let chat_options = self.get_options(ctx, &conversation.agent.model(), options)?;
        let chat_request = self
            .get_chat_request(conversation)
            .with_tools(self.get_tools(ctx, conversation)?);

        let chat_response = client
            .exec_chat(
                &conversation.agent.model().id(),
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

        if let Some(mut assistant_answer) = chat_response.first_text() {
            // OpenAI response does not respect stop support, so we need to manually truncate the response if stop sequences are provided
            if conversation.agent.model().adapter() == ModelAdapter::OpenAIResp
                && options.stop_sequences.len() > 0
            {
                for stop_sequence in &options.stop_sequences {
                    if let Some(index) = assistant_answer.find(stop_sequence) {
                        assistant_answer = &assistant_answer[..index];
                    }
                }
            }

            conversation.add_assistant_message(
                conversation.agent.metadata.name.clone(),
                conversation.agent.metadata.display_name.clone(),
                assistant_answer.to_string(),
            );
        };

        self.tool_calls_to_tool_calls(ctx, chat_response.into_tool_calls())
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
