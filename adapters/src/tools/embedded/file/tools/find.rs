// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::os::unix::fs::FileTypeExt;
use walkdir::WalkDir;

use super::super::permissions;
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    tools::{
        models::{Call, Schema, SchemaField, SchemaFieldType, ToolPermission},
        traits::Tool,
    },
};

const NAME: &str = "file_find";
const DESCRIPTION: &str = "Finds files and directories in a directory hierarchy.";
const MAX_DEPTH: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum SortBy {
    #[default]
    Name,
    Size,
    Modified,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum FileType {
    File,
    Directory,
    Symlink,
    Device,
    Fifo,
    Socket,

    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct Arguments {
    pub path: String,
    pub pattern: Option<String>,
    pub file_type: Option<FileType>,
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

struct FileEntry {
    pub path: String,
    pub size: u64,
    pub modified: u64,
    pub file_type: FileType,
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
                "Optional, only list files or directories, defaults to both.",
                Some(vec!["file".to_string(), "directory".to_string()]),
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
        let pattern = match args.pattern {
            Some(ref p) => Some(Regex::new(p).map_err(|e| {
                app_error!(
                    Validation,
                    "tool_failed",
                    &format!("Invalid regex pattern '{}': {}", p, e),
                    ctx.clone()
                )
            })?),
            None => None,
        };
        let depth = if args.recursive { MAX_DEPTH } else { 1 };
        let mut entries = walk_directory(
            ctx,
            &path.to_string_lossy(),
            pattern,
            args.file_type,
            depth,
            &args.exclude,
        )
        .await?;

        if entries.is_empty() {
            return Ok("No files or directories found.".to_string());
        }

        if args.sort_by == SortBy::Name {
            entries.sort_by(|a, b| a.path.cmp(&b.path));
        } else if args.sort_by == SortBy::Size {
            entries.sort_by(|a, b| a.size.cmp(&b.size));
        } else if args.sort_by == SortBy::Modified {
            entries.sort_by(|a, b| a.modified.cmp(&b.modified));
        }

        if args.descending {
            entries.reverse();
        }

        let limited_entries = entries.into_iter().take(args.max_results as usize);

        let result: Vec<String> = limited_entries
            .map(|entry| {
                format!(
                    "{} {} {} {:?}",
                    entry.path.strip_prefix(&ctx.chroot).unwrap_or("/"),
                    entry.size,
                    entry.modified,
                    entry.file_type
                )
            })
            .collect();

        Ok("Path Size Modified Type\n".to_string() + &result.join("\n"))
    }
}

fn is_excluded(entry: &walkdir::DirEntry, exclude: &[String]) -> bool {
    let name = entry.file_name().to_string_lossy();
    exclude.iter().any(|excluded| excluded == name.as_ref())
}

async fn walk_directory(
    ctx: &AppContext,
    path: &str,
    pattern: Option<Regex>,
    file_type: Option<FileType>,
    depth: usize,
    exclude: &[String],
) -> Result<Vec<FileEntry>, AppError> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(path)
        .max_depth(depth)
        .into_iter()
        .filter_entry(|e| !is_excluded(e, exclude))
    {
        let Ok(entry) = entry else {
            continue;
        };

        if let Some(file_type) = file_type {
            match file_type {
                FileType::File => {
                    if !entry.file_type().is_file() {
                        continue;
                    }
                }
                FileType::Directory => {
                    if !entry.file_type().is_dir() {
                        continue;
                    }
                }
                FileType::Symlink => {
                    if !entry.file_type().is_symlink() {
                        continue;
                    }
                }
                FileType::Device => {
                    if !entry.file_type().is_block_device() && !entry.file_type().is_char_device() {
                        continue;
                    }
                }
                FileType::Fifo => {
                    if !entry.file_type().is_fifo() {
                        continue;
                    }
                }
                FileType::Socket => {
                    if !entry.file_type().is_socket() {
                        continue;
                    }
                }
                _ => {
                    return Err(app_error!(
                        Validation,
                        "tool_failed",
                        &format!("Unsupported file type filter: {:?}", file_type),
                        ctx.clone()
                    ));
                }
            }
        }

        if let Some(pattern) = &pattern {
            if !pattern.is_match(&entry.file_name().to_string_lossy()) {
                continue;
            }
        }

        let file_entry = entry_to_file_entry(ctx, &entry).await?;
        entries.push(file_entry);
    }

    Ok(entries)
}

async fn entry_to_file_entry(
    ctx: &AppContext,
    entry: &walkdir::DirEntry,
) -> Result<FileEntry, AppError> {
    let metadata = entry.metadata().map_err(|e| {
        app_error!(
            Internal,
            "tool_failed",
            &format!(
                "Failed to get metadata for entry '{}': {}",
                entry.path().display(),
                e
            ),
            ctx.clone()
        )
    })?;

    let modified = metadata
        .modified()
        .map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!(
                    "Failed to get modified time for entry '{}': {}",
                    entry.path().display(),
                    e
                ),
                ctx.clone()
            )
        })?
        .elapsed()
        .map_err(|e| {
            app_error!(
                Internal,
                "tool_failed",
                &format!(
                    "Failed to get elapsed time for entry '{}': {}",
                    entry.path().display(),
                    e
                ),
                ctx.clone()
            )
        })?
        .as_secs();

    let file_type = if entry.file_type().is_file() {
        FileType::File
    } else if entry.file_type().is_dir() {
        FileType::Directory
    } else if entry.file_type().is_symlink() {
        FileType::Symlink
    } else if entry.file_type().is_block_device() || entry.file_type().is_char_device() {
        FileType::Device
    } else if entry.file_type().is_fifo() {
        FileType::Fifo
    } else if entry.file_type().is_socket() {
        FileType::Socket
    } else {
        return Err(app_error!(
            Validation,
            "tool_failed",
            &format!(
                "Unsupported file type for entry '{}'",
                entry.path().display()
            ),
            ctx.clone()
        ));
    };

    Ok(FileEntry {
        path: entry.path().to_string_lossy().to_string(),
        size: metadata.len(),
        modified,
        file_type,
    })
}
