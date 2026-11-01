// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::permissions;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError, file},
    inference::models::Conversation,
    tools::{
        models::{Call, Schema, SchemaField, SchemaFieldType, ToolPermission},
        traits::Tool,
    },
};

const NAME: &str = "file_find";
const DESCRIPTION: &str = "Finds files and directories in a directory hierarchy.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum SortBy {
    #[default]
    Name,
    Size,
    Modified,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct Arguments {
    pub path: String,
    pub pattern: Option<String>,
    pub file_type: Option<file::Kind>,
    pub sort_by: SortBy,
    pub descending: bool,
    pub recursive: bool,
    pub max_results: u64,
    pub exclude: Vec<String>,
}
impl Default for Arguments {
    fn default() -> Self {
        Arguments {
            path: "/".to_string(),
            pattern: None,
            file_type: None,
            sort_by: SortBy::Name,
            descending: false,
            recursive: true,
            max_results: 1000,
            exclude: vec![".git".to_string()],
        }
    }
}

struct Found {
    pub path: String,
    pub size: u64,
    pub modified: u64,
    pub file_type: file::Kind,
}

pub struct Find;

#[async_trait]
impl Tool for Find {
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
                "Absolute path within the current chroot to directory to find in.",
                None,
            ),
        );

        schema.add_property(
            "pattern",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "Optional, only list directories and files with a name matching provided regex.",
                None,
            ),
        );

        schema.add_property(
            "file_type",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "Optional, only list entries of this type, defaults to all. Symlinks are not followed, `other` covers devices, fifos and sockets.",
                Some(vec![
                    "file".to_string(),
                    "directory".to_string(),
                    "symlink".to_string(),
                    "other".to_string(),
                ]),
            ),
        );

        schema.add_property(
            "sort_by",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "Sort findings by, defaults to name.",
                Some(vec![
                    "name".to_string(),
                    "size".to_string(),
                    "modified".to_string(),
                ]),
            ),
        );

        schema.add_property(
            "descending",
            false,
            SchemaField::new(
                SchemaFieldType::Boolean,
                "Sort in descending order, defaults to false.",
                None,
            ),
        );

        schema.add_property(
            "recursive",
            false,
            SchemaField::new(
                SchemaFieldType::Boolean,
                "Recursively find files and directories, defaults to true.",
                None,
            ),
        );

        schema.add_property(
            "max_results",
            false,
            SchemaField::new(
                SchemaFieldType::Integer,
                "Maximum number of results to return, defaults to 1000.",
                None,
            ),
        );

        let mut exclude = SchemaField::new(
            SchemaFieldType::Array,
            "Names of files or directories to exclude from the search (matched against each path segment's basename); matching directories are not descended into. Defaults to ['.git']. Pass an empty array to include everything.",
            None,
        );
        exclude.set_items(
            _ctx,
            SchemaField::new(SchemaFieldType::String, "Name to exclude.", None),
        )?;
        schema.add_property("exclude", false, exclude);

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
        let pattern = match args.pattern {
            Some(ref p) => Some(Regex::new(p).map_err(|e| {
                app_error!(
                    Validation,
                    "tool_failed",
                    &format!("Invalid regex pattern '{p}': {e}"),
                    ctx.clone()
                )
            })?),
            None => None,
        };

        let path_order = args.sort_by == SortBy::Name && !args.descending;
        let depth = if args.recursive {
            0..=usize::MAX
        } else {
            0..=1
        };
        let max_results = usize::try_from(args.max_results).unwrap_or(usize::MAX);
        let mut entries: Vec<Found> = conversation
            .chroot()
            .find(
                ctx,
                path.as_str(),
                pattern.as_ref(),
                args.file_type.as_slice(),
                depth,
                &args.exclude,
                path_order.then_some(max_results),
            )
            .await?
            .iter()
            .map(Found::new)
            .collect();

        if entries.is_empty() {
            return Ok("No files or directories found.".to_string());
        }

        if args.sort_by == SortBy::Size {
            entries.sort_by_key(|a| a.size);
        } else if args.sort_by == SortBy::Modified {
            entries.sort_by_key(|a| a.modified);
        }

        if args.descending {
            entries.reverse();
        }

        let result: Vec<String> = entries
            .into_iter()
            .take(max_results)
            .map(|entry| {
                format!(
                    "{} {} {} {:?}",
                    entry.path, entry.size, entry.modified, entry.file_type
                )
            })
            .collect();

        Ok("Path Size Modified Type\n".to_string() + &result.join("\n"))
    }
}

impl Found {
    fn new(entry: &file::Entry) -> Self {
        Self {
            path: entry.path.to_string(),
            size: entry.metadata.size,
            modified: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_secs())
                .saturating_sub(entry.metadata.modified),
            file_type: entry.metadata.kind,
        }
    }
}
