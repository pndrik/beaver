// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use super::{Repository, hardening};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError, LiteralOrRegex},
};

const GIT_REPOSITORIES_CONFIGURATION_KEY: &str = "tools.configuration.git.repositories";

async fn get_repositories(ctx: &AppContext) -> Result<Vec<Repository>, AppError> {
    ctx.configuration()
        .get_json_value(ctx, GIT_REPOSITORIES_CONFIGURATION_KEY)
        .await?
        .as_array()
        .ok_or_else(|| {
            app_error!(
                Validation,
                "configuration_load_failed",
                &format!(
                    "Repository configuration is not an array: {GIT_REPOSITORIES_CONFIGURATION_KEY}"
                ),
                ctx.clone()
            )
        })?
        .iter()
        .map(|value| {
            serde_json::from_value::<Repository>(value.clone()).map_err(|e| {
                app_error!(
                    Validation,
                    "configuration_load_failed",
                    &format!("Failed to deserialize MCP server configuration: {e}"),
                    ctx.clone()
                )
            })
        })
        .collect::<Result<Vec<Repository>, AppError>>()
}

pub(super) async fn get_repository_by_url(
    ctx: &AppContext,
    url: &str,
) -> Result<Repository, AppError> {
    hardening::validate_url(url)
        .map_err(|message| app_error!(Validation, "tool_failed", &message, ctx.clone()))?;
    let repositories = get_repositories(ctx).await?;

    let patterns = repositories
        .iter()
        .map(|repo| {
            LiteralOrRegex::parse(&repo.url).map_err(|message| {
                app_error!(
                    Validation,
                    "configuration_load_failed",
                    &format!("Repository URL '{}' {message}", repo.url),
                    ctx.clone()
                )
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    let mut repository = repositories
        .into_iter()
        .zip(patterns)
        .find_map(|(repo, pattern)| pattern.matches(url).then_some(repo))
        .ok_or_else(|| {
            app_error!(
                NotFound,
                "configuration_load_failed",
                &format!("No configuration for repository: {url}"),
                ctx.clone()
            )
        })?;

    repository.url = url.to_string();

    Ok(repository)
}
