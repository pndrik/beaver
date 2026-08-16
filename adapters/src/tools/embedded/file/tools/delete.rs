// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;
use std::{fs, path::Path};

use super::super::permissions;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::{
        models::{Call, Schema, SchemaField, SchemaFieldType, ToolPermission},
        traits::Tool,
    },
};

const NAME: &str = "file_delete";
const DESCRIPTION: &str =
    "Deletes files and directories on the filesystem (directories are deleted recursively)";

#[derive(Debug, Deserialize)]
struct Arguments {
    pub path: String,
}

pub struct Delete;

#[async_trait]
impl Tool for Delete {
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
                "Absolute path within the current chroot to file or directory.",
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

        let path = permissions::absolute_path(ctx, &permissions, &args.path)?;

        delete_path(ctx, &path)
    }
}

fn delete_path(ctx: &AppContext, path: &Path) -> Result<String, AppError> {
    let metadata = fs::metadata(path).map_err(|e| {
        app_error!(
            Internal,
            "tool_failed",
            &format!("Failed to access path '{}': {}", path.display(), e),
            ctx.clone()
        )
    })?;

    if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!("Failed to delete directory '{}': {}", path.display(), e),
                ctx.clone()
            )
        })?;

        Ok(format!("Deleted directory '{}'", path.display()))
    } else {
        fs::remove_file(path).map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!("Failed to delete file '{}': {}", path.display(), e),
                ctx.clone()
            )
        })?;

        Ok(format!("Deleted file '{}'", path.display()))
    }
}
