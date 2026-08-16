// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_domains::core::models::{AppContext, AppError};
use app_domains::{inference::Inference, tools::Tools};

use crate::resolvers;

pub struct Domains {
    pub inference: Arc<Inference>,
    pub tools: Arc<Tools>,
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

async fn get_tools(ctx: &AppContext) -> Result<Tools, AppError> {
    let tools_providers = resolvers::tools_provider(ctx).await?;

    Tools::new(tools_providers)
}

impl Domains {
    pub async fn new(ctx: &AppContext) -> Result<Self, AppError> {
        Ok(Self {
            inference: Arc::new(get_inference(ctx).await?),
            tools: Arc::new(get_tools(ctx).await?),
        })
    }
}
