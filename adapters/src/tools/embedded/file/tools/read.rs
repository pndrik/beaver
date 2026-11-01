// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use serde::Deserialize;

use super::super::{permissions, read_state::State};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::models::Conversation,
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
        let state = State::new(ctx, chroot, &path, conversation).await?;
        let content = chroot.read_to_string(ctx, path.as_str()).await?;
        state.mark_read(conversation);
        Ok(lines(&content, args.start, args.end))
    }
}

fn lines(content: &str, start: Option<u64>, end: Option<u64>) -> String {
    if start.is_none() && end.is_none() {
        return content.to_string();
    }

    let start = start.unwrap_or(1).max(1);
    let end = end.unwrap_or(u64::MAX);
    content
        .lines()
        .skip(usize::try_from(start - 1).unwrap_or(usize::MAX))
        .take(usize::try_from(end.saturating_sub(start - 1)).unwrap_or(usize::MAX))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::lines;

    const FILE: &str = "one\ntwo\nthree\n";

    #[test]
    fn reads_line_ranges() {
        assert_eq!(lines(FILE, None, None), FILE);
        assert_eq!(lines(FILE, Some(2), None), "two\nthree");
        assert_eq!(lines(FILE, Some(2), Some(2)), "two");
        assert_eq!(lines(FILE, None, Some(2)), "one\ntwo");
        assert_eq!(lines(FILE, Some(3), Some(1)), "");
        assert_eq!(lines(FILE, Some(9), None), "");
    }
}
