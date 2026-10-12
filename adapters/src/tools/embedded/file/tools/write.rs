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
                "Optional, start writing from this line number (1-based). Use the line after the last line to add content at the end of the file.",
                None,
            ),
        );

        schema.add_property(
            "length",
            false,
            SchemaField::new(
                SchemaFieldType::Integer,
                "Optional, number of lines to replace starting from the start line number. Defaults to 0, which inserts the content before the start line without replacing anything.",
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
    let new_content = replace_lines(&original, start, length, content)
        .map_err(|message| app_error!(Validation, "skill_failed", &message, ctx.clone()))?;

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

fn replace_lines(
    original: &str,
    start: u64,
    length: Option<u64>,
    content: &str,
) -> Result<String, String> {
    let original_lines: Vec<&str> = original.lines().collect();
    let line_count = original_lines.len();

    if start == 0 {
        return Err("Start line number must be greater than 0".to_string());
    }
    let start_index = (start - 1) as usize;
    if start_index > line_count {
        return Err(format!(
            "Start line {} is beyond the end of the file ({} lines). Use a start line between 1 and {} or set append to true.",
            start,
            line_count,
            line_count + 1
        ));
    }
    let end_index = start_index
        .saturating_add(length.unwrap_or(0) as usize)
        .min(line_count);

    let new_lines: Vec<&str> = original_lines[..start_index]
        .iter()
        .copied()
        .chain(content.lines())
        .chain(original_lines[end_index..].iter().copied())
        .collect();

    let line_ending = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let trailing_newline = if original.is_empty() {
        content.ends_with('\n')
    } else {
        original.ends_with('\n')
    };

    let mut new_content = new_lines.join(line_ending);
    if trailing_newline && !new_lines.is_empty() {
        new_content.push_str(line_ending);
    }

    Ok(new_content)
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

#[cfg(test)]
mod tests {
    use super::replace_lines;

    const FILE: &str = "one\ntwo\nthree\n";

    #[test]
    fn replaces_lines() {
        assert_eq!(
            replace_lines(FILE, 2, Some(1), "TWO").unwrap(),
            "one\nTWO\nthree\n"
        );
    }

    #[test]
    fn replaces_multiple_lines_with_fewer_lines() {
        assert_eq!(replace_lines(FILE, 1, Some(2), "X").unwrap(), "X\nthree\n");
    }

    #[test]
    fn clamps_length_to_end_of_file() {
        assert_eq!(replace_lines(FILE, 2, Some(100), "X").unwrap(), "one\nX\n");
    }

    #[test]
    fn inserts_without_length() {
        assert_eq!(
            replace_lines(FILE, 2, None, "X").unwrap(),
            "one\nX\ntwo\nthree\n"
        );
    }

    #[test]
    fn inserts_with_zero_length() {
        assert_eq!(
            replace_lines(FILE, 1, Some(0), "X").unwrap(),
            "X\none\ntwo\nthree\n"
        );
    }

    #[test]
    fn appends_one_line_past_end() {
        assert_eq!(
            replace_lines(FILE, 4, None, "four").unwrap(),
            "one\ntwo\nthree\nfour\n"
        );
    }

    #[test]
    fn rejects_start_beyond_end_of_file() {
        let err = replace_lines(FILE, 96, Some(2), "X").unwrap_err();
        assert!(err.contains("beyond the end of the file (3 lines)"));
        assert!(err.contains("between 1 and 4"));
    }

    #[test]
    fn rejects_start_zero() {
        assert!(replace_lines(FILE, 0, None, "X").is_err());
    }

    #[test]
    fn writes_into_empty_file() {
        assert_eq!(replace_lines("", 1, None, "X\n").unwrap(), "X\n");
        assert_eq!(replace_lines("", 1, Some(5), "X").unwrap(), "X");
        assert!(replace_lines("", 2, None, "X").is_err());
    }

    #[test]
    fn keeps_missing_trailing_newline() {
        assert_eq!(
            replace_lines("one\ntwo", 2, Some(1), "TWO").unwrap(),
            "one\nTWO"
        );
    }

    #[test]
    fn keeps_crlf_line_endings() {
        assert_eq!(
            replace_lines("one\r\ntwo\r\n", 2, Some(1), "TWO").unwrap(),
            "one\r\nTWO\r\n"
        );
    }

    #[test]
    fn deleting_all_lines_leaves_empty_file() {
        assert_eq!(replace_lines(FILE, 1, Some(3), "").unwrap(), "");
    }
}
