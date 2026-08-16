// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::sync::Arc;

use crate::inference::GenAi;
use app_domains::{
    core::models::{AppContext, AppError},
    inference::{
        models::{Conversation, Model, Options},
        traits::InferenceProvider,
    },
    tools::models::Call as SkillCall,
};

mod helpers;

const CONFIGURATION_ENDPOINT: &str = "inference.providers.anthropic.endpoint";
const CONFIGURATION_APIKEY: &str = "inference.providers.anthropic.apikey";

pub struct Anthropic {
    genai: Arc<GenAi>,
}

impl Anthropic {
    pub fn new(ctx: &AppContext) -> Self {
        Self {
            genai: Arc::new(GenAi::new(
                ctx,
                CONFIGURATION_ENDPOINT,
                CONFIGURATION_APIKEY,
            )),
        }
    }
}

#[async_trait]
impl InferenceProvider for Anthropic {
    async fn infer(
        &self,
        ctx: &AppContext,
        options: &Options,
        conversation: &mut Conversation,
    ) -> Result<Vec<SkillCall>, AppError> {
        self.genai.infer(ctx, options, conversation).await
    }

    async fn supported_models(&self, ctx: &AppContext) -> Result<Vec<Model>, AppError> {
        let models = self.list_available_models(ctx).await?;

        Ok(models
            .into_iter()
            .filter_map(|model_name| Model::from_id(&model_name))
            .collect::<Vec<Model>>())
    }

    async fn supports_model(&self, ctx: &AppContext, model: &Model) -> Result<bool, AppError> {
        let models = self.list_available_models(ctx).await?;

        Ok(models.contains(&model.id().to_string()))
    }
}
