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

const NAME: &str = "git_show";
const DESCRIPTION: &str = "Shows a commit: message, changed files, diff against its first parent.";

const MAX_LINES: usize = 1000;

#[derive(Debug, Deserialize)]
struct Arguments {
    pub path: String,
    pub revision: String,
    #[serde(default)]
    pub file: Option<String>,
}

pub struct Show;

#[async_trait]
impl Tool for Show {
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
            true,
            SchemaField::new(
                SchemaFieldType::String,
                "Commit to show (commit SHA, branch, tag, HEAD~N, etc.).",
                None,
            ),
        );

        schema.add_property(
            "file",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "Only show changes to this file or directory, relative to the repository root.",
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

        let commit = GitRepository::local(path_str)
            .show(&args.revision, args.file.as_deref())
            .map_err(|e| {
                app_error!(
                    Internal,
                    "tool_failed",
                    &format!(
                        "Failed to show '{}' in repository '{}': {}",
                        args.revision, args.path, e
                    ),
                    ctx.clone()
                )
            })?;

        Ok(super::truncate_lines(commit, MAX_LINES))
    }
}
