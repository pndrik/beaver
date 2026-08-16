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

const NAME: &str = "git_diff";
const DESCRIPTION: &str = "Shows a diff in an existing local repository: unstaged changes against the index when 'from' and 'to' are omitted, or the diff between two revisions (commit SHAs, branch names, tags, HEAD~N, etc.) when both are given.";

const MAX_DIFF_LINES: usize = 1000;

#[derive(Debug, Deserialize)]
struct Arguments {
    pub path: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
}

pub struct Diff;

#[async_trait]
impl Tool for Diff {
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
            "from",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "Revision to diff from (commit SHA, branch, tag, HEAD~N, etc.). Must be provided together with 'to'; omit both for unstaged changes.",
                None,
            ),
        );

        schema.add_property(
            "to",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "Revision to diff to (commit SHA, branch, tag, HEAD~N, etc.). Must be provided together with 'from'.",
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

        if args.from.is_some() != args.to.is_some() {
            return Err(app_error!(
                Validation,
                "tool_failed",
                &format!(
                    "Both 'from' and 'to' must be provided together, or neither, for tool '{}'",
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

        let diff = repository
            .diff(args.from.as_deref(), args.to.as_deref())
            .map_err(|e| {
                app_error!(
                    Internal,
                    "tool_failed",
                    &format!("Failed to diff repository '{}': {}", args.path, e),
                    ctx.clone()
                )
            })?;

        if diff.is_empty() {
            return Ok(format!(
                "No differences found in repository '{}'.",
                args.path
            ));
        }

        let lines: Vec<&str> = diff.lines().collect();
        if lines.len() > MAX_DIFF_LINES {
            let truncated = lines[..MAX_DIFF_LINES].join("\n");
            Ok(format!(
                "{}\n... (truncated, {} more lines)",
                truncated,
                lines.len() - MAX_DIFF_LINES
            ))
        } else {
            Ok(diff)
        }
    }
}
