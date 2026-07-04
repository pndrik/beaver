// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use crate::core::{models::AppError, traits::TemplateEngine};

pub mod models;
pub mod traits;
pub mod use_cases;

pub struct Inference {
    pub(crate) agent_provider: Arc<dyn traits::AgentProvider + Send + Sync>,
    pub(crate) inference_providers: Vec<Arc<dyn traits::InferenceProvider + Send + Sync>>,
    pub(crate) webhook_provider: Arc<dyn traits::WebhookProvider + Send + Sync>,
    pub(crate) template_engine: Arc<dyn TemplateEngine + Send + Sync>,
}

impl Inference {
    pub fn new(
        agent_provider: Arc<dyn traits::AgentProvider + Send + Sync>,
        inference_providers: Vec<Arc<dyn traits::InferenceProvider + Send + Sync>>,
        webhook_provider: Arc<dyn traits::WebhookProvider + Send + Sync>,
        template_engine: Arc<dyn TemplateEngine + Send + Sync>,
    ) -> Result<Self, AppError> {
        Ok(Self {
            agent_provider,
            inference_providers,
            webhook_provider,
            template_engine,
        })
    }
}
