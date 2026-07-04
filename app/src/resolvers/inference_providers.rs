// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::inference::{Anthropic, Zen};
use app_domains::{
    core::models::{AppContext, AppError},
    inference::traits::InferenceProvider,
};

const CONFIGURATION_PROVIDERS: &str = "inference.providers";

async fn add_provider_if_enabled(
    ctx: &AppContext,
    providers: &mut Vec<Arc<dyn InferenceProvider + Send + Sync>>,
    provider_name: &str,
    provider_constructor: impl Fn(&AppContext) -> Arc<dyn InferenceProvider + Send + Sync>,
) {
    if ctx
        .configuration
        .get_bool(
            ctx,
            format!("{}.{}.enabled", CONFIGURATION_PROVIDERS, provider_name).as_str(),
        )
        .await
        .unwrap_or(false)
    {
        providers.push(provider_constructor(ctx));
    }
}

pub async fn inference_providers(
    ctx: &AppContext,
) -> Result<Vec<Arc<dyn InferenceProvider + Send + Sync>>, AppError> {
    let mut providers: Vec<Arc<dyn InferenceProvider + Send + Sync>> = Vec::new();

    add_provider_if_enabled(ctx, &mut providers, "anthropic", |ctx| {
        Arc::new(Anthropic::new(ctx))
    })
    .await;
    add_provider_if_enabled(ctx, &mut providers, "zen", |ctx| Arc::new(Zen::new(ctx))).await;

    Ok(providers)
}
