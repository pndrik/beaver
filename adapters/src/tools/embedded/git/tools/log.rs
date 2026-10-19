// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;

use super::super::{git_repository::GitRepository, permissions};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::models::Conversation,
    tools::{
        models::{Call, Schema, SchemaField, SchemaFieldType, ToolPermission},
        traits::Tool,
    },
};

const NAME: &str = "git_log";
const DESCRIPTION: &str = "Lists commits, newest first: SHA, date, author, subject, changed files.";

const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 200;

#[derive(Debug, Deserialize)]
struct Arguments {
    pub path: String,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub skip: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

pub struct Log;

#[async_trait]
impl Tool for Log {
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
            "revision",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "Revision to start from (commit SHA, branch, tag, HEAD~N, etc.). Defaults to HEAD.",
                None,
            ),
        );

        schema.add_property(
            "file",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "Only list commits that changed this file or directory, relative to the repository root.",
                None,
            ),
        );

        schema.add_property(
            "skip",
            false,
            SchemaField::new(
                SchemaFieldType::Integer,
                "Number of commits to skip, for paging. Defaults to 0.",
                None,
            ),
        );

        schema.add_property(
            "limit",
            false,
            SchemaField::new(
                SchemaFieldType::Integer,
                &format!(
                    "Maximum number of commits to list. Defaults to {}, at most {}.",
                    DEFAULT_LIMIT, MAX_LIMIT
                ),
                None,
            ),
        );

        Ok(schema)
    }

    async fn call(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
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

        let path = permissions::existing_repo_path(ctx, conversation, &permissions, &args.path)?;
        let path_str = path.to_str().ok_or_else(|| {
            app_error!(
                Validation,
                "tool_failed",
                &format!("Path '{}' is not valid UTF-8", args.path),
                ctx.clone()
            )
        })?;

        let commits = GitRepository::local(path_str)
            .log(
                args.revision.as_deref().unwrap_or("HEAD"),
                args.file.as_deref(),
                args.skip.unwrap_or(0),
                args.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
            )
            .map_err(|e| {
                app_error!(
                    Internal,
                    "tool_failed",
                    &format!(
                        "Failed to list commits in repository '{}': {}",
                        args.path, e
                    ),
                    ctx.clone()
                )
            })?;

        if commits.is_empty() {
            return Ok("No commits found.".to_string());
        }

        Ok(commits.join("\n"))
    }
}
