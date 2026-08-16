// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;

use super::Repository;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
};

const GIT_REPOSITORIES_CONFIGURATION_KEY: &str = "tools.configuration.git.repositories";

async fn get_repositories(ctx: &AppContext) -> Result<Vec<Repository>, AppError> {
    ctx.configuration
        .get_json_value(ctx, GIT_REPOSITORIES_CONFIGURATION_KEY)
        .await?
        .as_array()
        .ok_or_else(|| {
            app_error!(
                Validation,
                "configuration_load_failed",
                &format!(
                    "Repository configuration is not an array: {}",
                    GIT_REPOSITORIES_CONFIGURATION_KEY
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
                    &format!(
                        "Failed to deserialize MCP server configuration: {}",
                        e.to_string()
                    ),
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
    let repositories = get_repositories(ctx).await?;

    let mut repository = repositories
        .into_iter()
        .find(|repo| {
            if repo.url.starts_with('^') && repo.url.ends_with('$') {
                return Regex::new(&repo.url)
                    .map(|re| re.is_match(url))
                    .unwrap_or(false);
            }

            repo.url == url
        })
        .ok_or_else(|| {
            app_error!(
                NotFound,
                "configuration_load_failed",
                &format!("No configuration for repository: {}", url),
                ctx.clone()
            )
        })?;

    repository.url = url.to_string();

    Ok(repository)
}
