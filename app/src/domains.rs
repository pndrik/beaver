// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_domains::core::models::{AppContext, AppError};
use app_domains::{flows::Flows, inference::Inference, tools::Tools};

use crate::resolvers;

pub struct Domains {
    pub flows: Arc<Flows>,
    pub inference: Arc<Inference>,
    pub tools: Arc<Tools>,
}

async fn get_inference(ctx: &AppContext) -> Result<Inference, AppError> {
    let agent_provider = resolvers::agent_provider(ctx).await?;
    let inference_providers = resolvers::inference_providers(ctx).await?;

    Inference::new(agent_provider, inference_providers)
}

async fn get_flows(ctx: &AppContext) -> Result<Flows, AppError> {
    let flow_provider = resolvers::flow_provider(ctx).await?;
    let template_engine = resolvers::template_engine(ctx).await?;

    Flows::new(flow_provider, template_engine)
}

async fn get_tools(ctx: &AppContext) -> Result<Tools, AppError> {
    let tools_providers = resolvers::tools_provider(ctx).await?;
    let output_writer = resolvers::output_writer(ctx).await?;

    Tools::new(tools_providers, output_writer)
}

impl Domains {
    pub async fn new(ctx: &AppContext) -> Result<Self, AppError> {
        Ok(Self {
            flows: Arc::new(get_flows(ctx).await?),
            inference: Arc::new(get_inference(ctx).await?),
            tools: Arc::new(get_tools(ctx).await?),
        })
    }
}
