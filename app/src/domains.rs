// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_domains::core::models::{AppContext, AppError};
use app_domains::{inference::Inference, skills::Skills};

use crate::resolvers;

pub struct Domains {
    pub inference: Arc<Inference>,
    pub skills: Arc<Skills>,
}

async fn get_inference(ctx: &AppContext) -> Result<Inference, AppError> {
    let agent_provider = resolvers::agent_provider(ctx).await?;
    let inference_providers = resolvers::inference_providers(ctx).await?;
    let webhook_provider = resolvers::webhook_provider(ctx).await?;
    let template_engine = resolvers::template_engine(ctx).await?;

    Inference::new(
        agent_provider,
        inference_providers,
        webhook_provider,
        template_engine,
    )
}

async fn get_skills(ctx: &AppContext) -> Result<Skills, AppError> {
    let skills_providers = resolvers::skills_provider(ctx).await?;

    Skills::new(skills_providers)
}

impl Domains {
    pub async fn new(ctx: &AppContext) -> Result<Self, AppError> {
        Ok(Self {
            inference: Arc::new(get_inference(ctx).await?),
            skills: Arc::new(get_skills(ctx).await?),
        })
    }
}
