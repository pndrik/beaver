// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use std::{fs::File, io::Read};
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

const NAME: &str = "file_search";
const DESCRIPTION: &str =
    "Recursively searches files (max 10MB) for content matching a regex pattern.";
const MAX_DEPTH: usize = 1000;
const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_LINE_LENGTH_CHARS: usize = 100;

#[derive(Debug, Deserialize)]
#[serde(default)]
struct Arguments {
    pub path: String,
    pub pattern: String,
    pub recursive: bool,
    pub max_results: u64,
    pub exclude: Vec<String>,
}
impl Default for Arguments {
    fn default() -> Self {
        Arguments {
            path: "/".to_string(),
            pattern: String::new(),
            recursive: true,
            max_results: 1000,
            exclude: vec![".git".to_string()],
        }
    }
}

struct MatchEntry {
    pub path: String,
    pub line_number: usize,
    pub line: String,
}

pub struct Search;

#[async_trait]
impl Tool for Search {
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
                "Absolute path within the current chroot to directory to search in.",
                None,
            ),
        );

        schema.add_property(
            "pattern",
            true,
            SchemaField::new(
                SchemaFieldType::String,
                "Regex pattern to search for in file contents.",
                None,
            ),
        );

        schema.add_property(
            "recursive",
            false,
            SchemaField::new(
                SchemaFieldType::Boolean,
                "Recursively search files and directories, defaults to true.",
                None,
            ),
        );

        schema.add_property(
            "max_results",
            false,
            SchemaField::new(
                SchemaFieldType::Integer,
                "Maximum number of matching lines to return, defaults to 1000.",
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

        if args.pattern.trim().is_empty() {
            return Err(app_error!(
                Validation,
                "tool_failed",
                "Argument 'pattern' must not be empty.",
                ctx.clone()
            ));
        }

        let path = permissions::absolute_path(ctx, &permissions, &args.path)?;
        let regex = Regex::new(&args.pattern).map_err(|e| {
            app_error!(
                Validation,
                "tool_failed",
                &format!("Invalid regex pattern '{}': {}", args.pattern, e),
                ctx.clone()
            )
        })?;

        let depth = if args.recursive { MAX_DEPTH } else { 1 };
        let matches = search_directory_contents(
            ctx,
            &path.to_string_lossy(),
            &regex,
            depth,
            args.max_results,
            &args.exclude,
        )
        .await?;

        if matches.is_empty() {
            return Ok("No matching file content found.".to_string());
        }

        let result: Vec<String> = matches
            .into_iter()
            .map(|m| {
                format!(
                    "{}:{} {}",
                    m.path.strip_prefix(&ctx.chroot).unwrap_or("/"),
                    m.line_number,
                    m.line
                )
            })
            .collect();

        Ok("Path:Line Match\n".to_string() + &result.join("\n"))
    }
}

fn is_excluded(entry: &walkdir::DirEntry, exclude: &[String]) -> bool {
    let name = entry.file_name().to_string_lossy();
    exclude.iter().any(|excluded| excluded == name.as_ref())
}

async fn search_directory_contents(
    _ctx: &AppContext,
    path: &str,
    pattern: &Regex,
    depth: usize,
    max_results: u64,
    exclude: &[String],
) -> Result<Vec<MatchEntry>, AppError> {
    let mut matches = Vec::new();

    for entry in WalkDir::new(path)
        .max_depth(depth)
        .into_iter()
        .filter_entry(|e| !is_excluded(e, exclude))
    {
        let Ok(entry) = entry else {
            continue;
        };

        if !entry.file_type().is_file() {
            continue;
        }

        if let Ok(metadata) = entry.metadata() {
            if metadata.len() > MAX_FILE_SIZE_BYTES {
                continue;
            }
        }

        let mut file = match File::open(entry.path()) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let mut content = String::new();
        if file.read_to_string(&mut content).is_err() {
            continue;
        }

        for (idx, line) in content.lines().enumerate() {
            if pattern.is_match(line) {
                let mut truncated_line: String = line.chars().take(MAX_LINE_LENGTH_CHARS).collect();
                if line.chars().count() > MAX_LINE_LENGTH_CHARS {
                    truncated_line.push_str("... [truncated]");
                }

                matches.push(MatchEntry {
                    path: entry.path().to_string_lossy().to_string(),
                    line_number: idx + 1,
                    line: truncated_line,
                });

                if matches.len() >= max_results as usize {
                    return Ok(matches);
                }
            }
        }
    }

    Ok(matches)
}
