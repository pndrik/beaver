// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write as _},
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

const NAME: &str = "file_write";
const DESCRIPTION: &str = "Writes a file to the filesystem";

#[derive(Debug, Deserialize)]
#[serde(default)]
struct Arguments {
    pub path: String,
    pub start: Option<u64>,
    pub length: Option<u64>,
    pub append: bool,
    pub content: String,
}
impl Default for Arguments {
    fn default() -> Self {
        Arguments {
            path: "/".to_string(),
            start: None,
            length: None,
            append: false,
            content: "".to_string(),
        }
    }
}

pub struct Write;

#[async_trait]
impl Tool for Write {
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
                "Optional, start writing from this line number (1-based).",
                None,
            ),
        );

        schema.add_property(
            "length",
            false,
            SchemaField::new(
                SchemaFieldType::Integer,
                "Optional, number of lines to replace starting from the start line number.",
                None,
            ),
        );

        schema.add_property(
            "append",
            false,
            SchemaField::new(
                SchemaFieldType::Boolean,
                "Append to the file instead of overwriting.",
                None,
            ),
        );

        schema.add_property(
            "content",
            true,
            SchemaField::new(
                SchemaFieldType::String,
                "Content to write to the file.",
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

        let path = Path::new(&args.path);
        let parent = path.parent().ok_or_else(|| {
            app_error!(
                Validation,
                "tool_failed",
                &format!("Path '{}' has no parent directory", path.display()),
                ctx.clone()
            )
        })?;
        let child = path.file_name().ok_or_else(|| {
            app_error!(
                Validation,
                "tool_failed",
                &format!("Path '{}' has no file name", path.display()),
                ctx.clone()
            )
        })?;
        let path_absolute =
            permissions::absolute_path(ctx, &permissions, &parent.to_string_lossy())?.join(child);

        write_file(
            ctx,
            &path_absolute,
            args.start,
            args.length,
            args.append,
            &args.content,
        )?;

        Ok(format!("Successfully wrote to file '{}'", path.display()))
    }
}

fn open_file_for_writing(
    ctx: &AppContext,
    path: &Path,
    start: Option<u64>,
    append: bool,
) -> Result<fs::File, AppError> {
    if path.is_dir() {
        return Err(app_error!(
            Validation,
            "skill_failed",
            &format!(
                "Path '{}' is a directory can not write to it",
                path.display()
            ),
            ctx.clone()
        ));
    }

    let mut open_options = fs::OpenOptions::new();
    open_options.create(true);
    if append {
        open_options.append(true);
    } else {
        open_options
            .read(true)
            .write(true)
            .truncate(start.is_none());
    }

    open_options.open(path).map_err(|e| {
        app_error!(
            ServiceUnavailable,
            "skill_failed",
            &format!("Failed to open file '{}': {}", path.display(), e),
            ctx.clone()
        )
    })
}

fn replace_lines_in_file(
    ctx: &AppContext,
    file: &mut fs::File,
    start: u64,
    length: Option<u64>,
    content: &str,
) -> Result<(), AppError> {
    let mut original = String::new();
    file.read_to_string(&mut original).map_err(|e| {
        app_error!(
            ServiceUnavailable,
            "skill_failed",
            &format!("Failed to read file for line replacement: {}", e),
            ctx.clone()
        )
    })?;
    let original_lines: Vec<&str> = original.lines().collect();

    let start = start as usize - 1;
    let end = match length {
        Some(len) => (start + len as usize).min(original_lines.len()) - 1,
        None => original_lines.len(),
    };

    let before = &original_lines[..start];
    let after = &original_lines[end + 1..];
    let content_lines: Vec<&str> = content.lines().collect();

    let new_lines: Vec<&str> = before
        .iter()
        .copied()
        .chain(content_lines.iter().copied())
        .chain(after.iter().copied())
        .collect();

    let new_content = new_lines.join("\n");

    file.seek(SeekFrom::Start(0)).map_err(|e| {
        app_error!(
            ServiceUnavailable,
            "skill_failed",
            &format!("Failed to seek in file for line replacement: {}", e),
            ctx.clone()
        )
    })?;
    file.set_len(0).map_err(|e| {
        app_error!(
            ServiceUnavailable,
            "skill_failed",
            &format!("Failed to truncate file for line replacement: {}", e),
            ctx.clone()
        )
    })?;
    file.write_all(new_content.as_bytes()).map_err(|e| {
        app_error!(
            ServiceUnavailable,
            "skill_failed",
            &format!("Failed to write to file for line replacement: {}", e),
            ctx.clone()
        )
    })?;

    Ok(())
}

fn write_file(
    ctx: &AppContext,
    path: &Path,
    start: Option<u64>,
    length: Option<u64>,
    append: bool,
    content: &str,
) -> Result<(), AppError> {
    let mut file = open_file_for_writing(ctx, path, start, append)?;

    if append {
        return file.write_all(content.as_bytes()).map_err(|e| {
            app_error!(
                ServiceUnavailable,
                "skill_failed",
                &format!("Failed to write to file '{}': {}", path.display(), e),
                ctx.clone()
            )
        });
    }

    if let Some(start) = start {
        if start == 0 {
            return Err(app_error!(
                Validation,
                "skill_failed",
                "Start line number must be greater than 0",
                ctx.clone()
            ));
        }

        return replace_lines_in_file(ctx, &mut file, start, length, content);
    }

    // If no start is provided, overwrite the entire file
    file.set_len(0).map_err(|e| {
        app_error!(
            ServiceUnavailable,
            "skill_failed",
            &format!("Failed to truncate file '{}': {}", path.display(), e),
            ctx.clone()
        )
    })?;
    file.write_all(content.as_bytes()).map_err(|e| {
        app_error!(
            ServiceUnavailable,
            "skill_failed",
            &format!("Failed to write to file '{}': {}", path.display(), e),
            ctx.clone()
        )
    })?;

    Ok(())
}
