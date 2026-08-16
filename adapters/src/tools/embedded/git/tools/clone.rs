// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;

use super::super::{configuration, git_repository::GitRepository, permissions};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::{
        models::{Call, Schema, SchemaField, SchemaFieldType, ToolPermission},
        traits::Tool,
    },
};

const NAME: &str = "git_clone";
const DESCRIPTION: &str = "Clones a Git repository into a directory within the current chroot.";

#[derive(Debug, Deserialize)]
struct Arguments {
    pub path: String,
    pub url: String,
}

pub struct Clone;

#[async_trait]
impl Tool for Clone {
    async fn name(&self, _ctx: &AppContext) -> Result<String, AppError> {
        Ok(NAME.to_string())
    }

    async fn description(&self, _ctx: &AppContext) -> Result<String, AppError> {
        Ok(DESCRIPTION.to_string())
    }

    async fn schema(&self, _ctx: &AppContext) -> Result<Schema, AppError> {
        let mut schema = Schema::new(NAME, "");

        schema.add_property(
            "path",
            true,
            SchemaField::new(
                SchemaFieldType::String,
                "Absolute path within the current chroot to clone the repository into. Must not already exist.",
                None,
            ),
        );

        schema.add_property(
            "url",
            true,
            SchemaField::new(
                SchemaFieldType::String,
                "URL of the Git repository to clone (ssh://... / https://...).",
                None,
            ),
        );

        Ok(schema)
    }

    async fn call(
        &self,
        ctx: &AppContext,
        permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        let args: Arguments = input.arguments_into().map_err(|e| {
            app_error!(
                Validation,
                "tool_failed",
                &format!("Invalid arguments for tool '{}': {}", NAME, e),
                ctx.clone()
            )
        })?;

        let path = permissions::clone_target_path(ctx, &permissions, &args.path)?;
        let path_str = path.to_str().ok_or_else(|| {
            app_error!(
                Validation,
                "tool_failed",
                &format!("Path '{}' is not valid UTF-8", args.path),
                ctx.clone()
            )
        })?;

        let repository = configuration::get_repository_by_url(ctx, &args.url).await?;
        let git_repository = GitRepository::new(repository, path_str);

        git_repository.clone().map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!("Failed to clone repository '{}': {}", args.url, e),
                ctx.clone()
            )
        })?;

        Ok(format!(
            "Cloned repository '{}' into '{}'",
            args.url, args.path
        ))
    }
}
