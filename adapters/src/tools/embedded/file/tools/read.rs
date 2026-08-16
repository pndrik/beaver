// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;
use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
};

use super::super::permissions;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::{
        models::{Call, Schema, SchemaField, SchemaFieldType, ToolPermission},
        traits::Tool,
    },
};

const NAME: &str = "file_read";
const DESCRIPTION: &str = "Reads files from the filesystem";

#[derive(Debug, Deserialize)]
struct Arguments {
    pub path: String,
    pub start: Option<u64>,
    pub end: Option<u64>,
}

pub struct Read;

#[async_trait]
impl Tool for Read {
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
                "Absolute path within the current chroot to file.",
                None,
            ),
        );

        schema.add_property(
            "start",
            false,
            SchemaField::new(
                SchemaFieldType::Integer,
                "Optional, start reading from this line number (1-based).",
                None,
            ),
        );

        schema.add_property(
            "end",
            false,
            SchemaField::new(
                SchemaFieldType::Integer,
                "Optional, end reading at this line number (1-based).",
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

        read_file(ctx, &path, args.start, args.end)
    }
}

fn read_file(
    ctx: &AppContext,
    path: &Path,
    start: Option<u64>,
    length: Option<u64>,
) -> Result<String, AppError> {
    if !path.is_file() {
        return Err(app_error!(
            Validation,
            "tool_failed",
            &format!("Path '{}' is not a file", path.display()),
            ctx.clone()
        ));
    }

    if start.is_none() && length.is_none() {
        return fs::read_to_string(path).map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!("Failed to read file '{}': {}", path.display(), e),
                ctx.clone()
            )
        });
    }

    let file = fs::File::open(path).map_err(|e| {
        app_error!(
            Internal,
            "tool_failed",
            &format!("Failed to open file '{}': {}", path.display(), e),
            ctx.clone()
        )
    })?;

    let mut out = String::new();
    let mut line_no = 1u64;
    let start = start.unwrap_or(1);
    let end = length.map(|l| start + l - 1);

    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!("Failed to read file '{}': {}", path.display(), e),
                ctx.clone()
            )
        })?;

        if line_no >= start {
            if let Some(e) = end {
                if line_no > e {
                    break;
                }
            }
            out.push_str(&line);
            out.push('\n');
        }

        line_no += 1;
    }
    _ = out.pop();

    Ok(out)
}
