// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;

use super::super::{permissions, read_state::State};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError, ErrorKind, file},
    inference::models::Conversation,
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
            content: String::new(),
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
                "Optional, start writing from this line number (1-based). Use the line after the last line to add content at the end of the file. Line edits require a current `file_read` of the file: read it first and again after other changes.",
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

        let path = permissions::resolve(ctx, conversation, &permissions, &args.path).await?;
        let chroot = conversation.chroot();
        let before = match State::new(ctx, chroot, &path, conversation).await {
            Err(err) if err.kind == ErrorKind::NotFound => None,
            state => Some(state?),
        };
        if args.start.is_some() && before.as_ref().is_some_and(|state| !state.is_current()) {
            return Err(app_error!(
                Validation,
                "file_not_read",
                format!(
                    "File '{path}' was not read or has changed since it was last read. Read it before editing lines."
                ),
                ctx.clone()
            ));
        }

        let (content, mode) = match (args.append, args.start) {
            (true, _) => (args.content, file::WriteMode::Append),
            (false, None) => (args.content, file::WriteMode::Truncate),
            (false, Some(start)) => {
                let original = match chroot.read_to_string(ctx, path.as_str()).await {
                    Err(err) if err.kind == ErrorKind::NotFound => String::new(),
                    original => original?,
                };
                let replaced = replace_lines(&original, start, args.length, &args.content)
                    .map_err(|message| {
                        app_error!(Validation, "skill_failed", &message, ctx.clone())
                    })?;
                (replaced, file::WriteMode::Truncate)
            }
        };

        chroot
            .write(ctx, path.as_str(), content.as_bytes(), mode)
            .await?;

        // After an append the whole content is only known if it was before.
        if !args.append || before.as_ref().is_some_and(State::is_current) {
            State::new(ctx, chroot, &path, conversation)
                .await?
                .mark_read(conversation);
        }

        Ok(format!("Successfully wrote to file '{}'", args.path))
    }
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
    let start_index = usize::try_from(start - 1).unwrap_or(usize::MAX);
    if start_index > line_count {
        return Err(format!(
            "Start line {} is beyond the end of the file ({} lines). Use a start line between 1 and {} or set append to true.",
            start,
            line_count,
            line_count + 1
        ));
    }
    let end_index = start_index
        .saturating_add(usize::try_from(length.unwrap_or(0)).unwrap_or(usize::MAX))
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
