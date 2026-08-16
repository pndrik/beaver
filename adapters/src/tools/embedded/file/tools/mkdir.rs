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

const NAME: &str = "file_mkdir";
const DESCRIPTION: &str = "Creates directories on the filesystem (like mkdir -p)";

#[derive(Debug, Deserialize)]
struct Arguments {
    pub path: String,
}

pub struct Mkdir;

#[async_trait]
impl Tool for Mkdir {
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
                "Absolute path within the current chroot to directory.",
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

        let path = permissions::absolute_path(ctx, &permissions, "/")?;
        let sub_path_sanitized = args.path.trim_start_matches('/');
        let full_path = path.join(sub_path_sanitized);

        if !permissions::check_scope(&permissions, full_path.to_str().unwrap_or("")) {
            return Err(app_error!(
                Unauthorized,
                "tool_failed",
                &format!(
                    "Path '{}' is not within allowed scopes: {:?}",
                    full_path.display(),
                    permissions.scopes
                ),
                ctx.clone()
            ));
        }

        create_directory(ctx, &full_path)?;

        Ok(format!(
            "Created directory '{}'",
            path.display()
                .to_string()
                .strip_prefix(&ctx.chroot)
                .unwrap_or("/")
        ))
    }
}

fn create_directory(ctx: &AppContext, path: &Path) -> Result<(), AppError> {
    fs::create_dir_all(path).map_err(|e| {
        app_error!(
            Internal,
            "tool_failed",
            &format!("Failed to create directory '{}': {}", path.display(), e),
            ctx.clone()
        )
    })?;

    Ok(())
}
