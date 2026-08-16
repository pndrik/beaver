// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;

use super::super::{Repository, configuration, git_repository::GitRepository, permissions};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::{
        models::{Call, Schema, SchemaField, SchemaFieldType, ToolPermission},
        traits::Tool,
    },
};

const NAME: &str = "git_branch_delete";
const DESCRIPTION: &str = "Deletes a local branch in an existing repository, optionally also deleting the matching branch on the remote.";

#[derive(Debug, Deserialize)]
struct Arguments {
    pub path: String,
    pub branch: String,
    #[serde(default)]
    pub delete_remote: bool,
    #[serde(default)]
    pub url: Option<String>,
}

pub struct BranchDelete;

#[async_trait]
impl Tool for BranchDelete {
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
                "Name of the local branch to delete.",
                None,
            ),
        );

        schema.add_property(
            "delete_remote",
            false,
            SchemaField::new(
                SchemaFieldType::Boolean,
                "If true, also deletes the branch on the remote. Defaults to false.",
                None,
            ),
        );

        schema.add_property(
            "url",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "URL of the Git repository; required only when 'delete_remote' is true.",
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

        if args.delete_remote && args.url.is_none() {
            return Err(app_error!(
                Validation,
                "tool_failed",
                &format!(
                    "The 'url' argument is required when 'delete_remote' is true for tool '{}'",
                    NAME
                ),
                ctx.clone()
            ));
        }

        let path = permissions::existing_repo_path(ctx, &permissions, &args.path)?;
        let path_str = path.to_str().ok_or_else(|| {
            app_error!(
                Validation,
                "tool_failed",
                &format!("Path '{}' is not valid UTF-8", args.path),
                ctx.clone()
            )
        })?;

        let repository = if args.delete_remote {
            configuration::get_repository_by_url(ctx, args.url.as_deref().unwrap()).await?
        } else {
            Repository {
                url: String::new(),
                email: String::new(),
                display_name: None,
                username: None,
                ssh_key: None,
                password: None,
                known_hosts: vec![],
            }
        };

        let git_repository = GitRepository::new(repository, path_str);

        if git_repository
            .is_current_branch(&args.branch)
            .unwrap_or(false)
        {
            return Err(app_error!(
                Conflict,
                "tool_failed",
                &format!(
                    "Cannot delete branch '{}': it is the currently checked out branch",
                    args.branch
                ),
                ctx.clone()
            ));
        }

        git_repository.delete_branch(&args.branch).map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!(
                    "Failed to delete branch '{}' in repository '{}': {}",
                    args.branch, args.path, e
                ),
                ctx.clone()
            )
        })?;

        if args.delete_remote {
            git_repository
                .delete_remote_branch(&args.branch)
                .map_err(|e| {
                    app_error!(
                        Internal,
                        "tool_failed",
                        &format!(
                            "Failed to delete remote branch '{}' in repository '{}': {}",
                            args.branch, args.path, e
                        ),
                        ctx.clone()
                    )
                })?;
        }

        Ok(format!(
            "Deleted branch '{}' in repository '{}'{}",
            args.branch,
            args.path,
            if args.delete_remote {
                " and on remote"
            } else {
                ""
            }
        ))
    }
}
