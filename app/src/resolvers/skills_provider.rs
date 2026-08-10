// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::skills::provider::EmbeddedProvider;
use app_domains::{
    core::models::{AppContext, AppError},
    skills::traits::SkillsProvider,
};

const CONFIGURATION_PROVIDERS: &str = "skills.providers";

async fn add_provider_if_enabled(
    ctx: &AppContext,
    providers: &mut Vec<Arc<dyn SkillsProvider + Send + Sync>>,
    provider_name: &str,
    provider_constructor: impl AsyncFn(
        &AppContext,
    ) -> Result<Arc<dyn SkillsProvider + Send + Sync>, AppError>,
) -> Result<(), AppError> {
    if ctx
        .configuration
        .get_bool(
            ctx,
            format!("{}.{}.enabled", CONFIGURATION_PROVIDERS, provider_name).as_str(),
        )
        .await?
    {
        providers.push(provider_constructor(ctx).await?);
    }

    Ok(())
}

pub async fn skills_provider(
    ctx: &AppContext,
) -> Result<Vec<Arc<dyn SkillsProvider + Send + Sync>>, AppError> {
    let mut providers: Vec<Arc<dyn SkillsProvider + Send + Sync>> = Vec::new();

    add_provider_if_enabled(ctx, &mut providers, "embedded", async |ctx| {
        Ok(Arc::new(EmbeddedProvider::new(ctx)) as Arc<dyn SkillsProvider + Send + Sync>)
    })
    .await?;

    add_provider_if_enabled(ctx, &mut providers, "javascript", async |ctx| {
        let mut provider = app_adapters::skills::provider::JavascriptProvider::new(ctx);
        provider.reload(ctx).await?;

        Ok(Arc::new(provider) as Arc<dyn SkillsProvider + Send + Sync>)
    })
    .await?;

    add_provider_if_enabled(ctx, &mut providers, "mcp", async |ctx| {
        let mut provider = app_adapters::skills::provider::McpProvider::new(ctx);
        provider.reload(ctx).await?;

        Ok(Arc::new(provider) as Arc<dyn SkillsProvider + Send + Sync>)
    })
    .await?;

    Ok(providers)
}
