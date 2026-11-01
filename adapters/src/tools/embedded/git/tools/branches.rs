// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;

use super::super::{Repository, git_repository::GitRepository, permissions};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::models::Conversation,
    tools::{
        models::{Call, Schema, SchemaField, SchemaFieldType, ToolPermission},
        traits::Tool,
    },
};

const NAME: &str = "git_branches";
const DESCRIPTION: &str =
    "Lists local branches in an existing repository, marking the currently checked out branch.";

#[derive(Debug, Deserialize)]
struct Arguments {
    pub path: String,
    #[serde(default)]
    pub include_remote: bool,
}

pub struct Branches;

#[async_trait]
impl Tool for Branches {
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
            "include_remote",
            false,
            SchemaField::new(
                SchemaFieldType::Boolean,
                "Also list remote-tracking branches, defaults to false.",
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
                &format!("Invalid arguments for tool '{NAME}': {e}"),
                ctx.clone()
            )
        })?;

        let local = permissions::repository(ctx, conversation, &permissions, &args.path).await?;

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
            local.path(),
        );

        let branches = repository.list_branches(args.include_remote).map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!(
                    "Failed to list branches in repository '{}': {}",
                    args.path, e
                ),
                ctx.clone()
            )
        })?;

        if branches.is_empty() {
            return Ok("No branches found.".to_string());
        }

        let result: Vec<String> = branches
            .into_iter()
            .map(|(name, is_current)| format!("{} {}", if is_current { "*" } else { " " }, name))
            .collect();

        Ok(result.join("\n"))
    }
}
