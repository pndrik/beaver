// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;

use super::super::{Repository, git_repository::GitRepository, permissions};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::{
        models::{Call, Schema, SchemaField, SchemaFieldType, ToolPermission},
        traits::Tool,
    },
};

const NAME: &str = "git_checkout";
const DESCRIPTION: &str = "Checks out a branch in an existing local repository, creating it locally from a matching remote-tracking branch or HEAD if needed.";

#[derive(Debug, Deserialize)]
struct Arguments {
    pub path: String,
    pub branch: String,
}

pub struct Checkout;

#[async_trait]
impl Tool for Checkout {
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
                "Absolute path within the current chroot to an existing repository.",
                None,
            ),
        );

        schema.add_property(
            "branch",
            true,
            SchemaField::new(
                SchemaFieldType::String,
                "Name of the branch to check out.",
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

        let path = permissions::existing_repo_path(ctx, &permissions, &args.path)?;
        let path_str = path.to_str().ok_or_else(|| {
            app_error!(
                Validation,
                "tool_failed",
                &format!("Path '{}' is not valid UTF-8", args.path),
                ctx.clone()
            )
        })?;

        let repository = GitRepository::new(
            Repository {
                url: String::new(),
                email: String::new(),
                display_name: None,
                username: None,
                ssh_key: None,
                password: None,
                known_hosts: vec![],
            },
            path_str,
        );

        repository.checkout(&args.branch).map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!(
                    "Failed to check out branch '{}' in repository '{}': {}",
                    args.branch, args.path, e
                ),
                ctx.clone()
            )
        })?;

        Ok(format!(
            "Checked out branch '{}' in repository '{}'",
            args.branch, args.path
        ))
    }
}
