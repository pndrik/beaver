// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::models::Conversation,
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

        let path = permissions.scoped_path(ctx, &args.path)?;
        conversation
            .chroot()
            .mkdir(ctx, path.as_str(), true)
            .await?;

        Ok(format!("Created directory '{path}'"))
    }
}
