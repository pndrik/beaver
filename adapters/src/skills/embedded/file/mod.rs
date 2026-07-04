// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    skills::{
        models::{Call, Schema, SchemaField, SchemaFieldType, SkillPermission},
        traits::Skill,
    },
};

mod arguments;
mod directories;
mod permissions;
mod read_file;
mod write_file;

pub struct File;
const NAME: &str = "file";
const DESCRIPTION: &str = "A skill for reading/writing files and listing directories.";
const MAX_FILES_LISTED: u64 = 1000;

struct Arguments {
    action: String,
    path: String,
    start: Option<u64>,
    length: Option<u64>,
    recursive: bool,
    append: bool,
    content: Option<String>,
}

#[async_trait]
impl Skill for File {
    async fn name(&self, _ctx: &AppContext) -> Result<String, AppError> {
        Ok(NAME.to_string())
    }

    async fn description(&self, _ctx: &AppContext) -> Result<String, AppError> {
        Ok(DESCRIPTION.to_string())
    }

    async fn schema(&self, _ctx: &AppContext) -> Result<Schema, AppError> {
        let mut schema = Schema::new(NAME, "");
        schema.add_property("action", true, SchemaField::new(
            SchemaFieldType::String,
            "Action to perform file and directory operations. Delete on a directory will delete all files and subdirectories.",
            Some(vec![
                "read".to_string(),
                "list".to_string(),
                "write".to_string(),
                "delete".to_string(),
                "mkdir".to_string(),
            ]),
        ))?;

        schema.add_property(
            "path",
            true,
            SchemaField::new(
                SchemaFieldType::String,
                "Path within the current chroot to the file or directory.",
                None,
            ),
        )?;
        schema.add_property(
            "start",
            false,
            SchemaField::new(
                SchemaFieldType::Integer,
                "Start line for reading/writing, if unset reads/overwrites from beginning.",
                None,
            ),
        )?;
        schema.add_property(
            "length",
            false,
            SchemaField::new(
                SchemaFieldType::Integer,
                "Number of lines to read/write, if unset reads/overwrites to end.",
                None,
            ),
        )?;
        schema.add_property(
            "recursive",
            false,
            SchemaField::new(
                SchemaFieldType::Boolean,
                "Whether to list files in subdirectories too.",
                None,
            ),
        )?;
        schema.add_property(
            "append",
            false,
            SchemaField::new(
                SchemaFieldType::Boolean,
                "Whether to append to the file instead of overwriting it (only for write action).",
                None,
            ),
        )?;
        schema.add_property(
            "content",
            false,
            SchemaField::new(
                SchemaFieldType::String,
                "Content to write to the file, only used for 'write' action.",
                None,
            ),
        )?;

        Ok(schema)
    }

    async fn execute(
        &self,
        ctx: &AppContext,
        permissions: SkillPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        let arguments = Self::skill_call_to_arguments(ctx, input)?;

        match arguments.action.as_str() {
            "read" => {
                if !permissions.roles.contains(&"read".to_string()) {
                    return Err(app_error!(
                        Unauthorized,
                        "skill_failed",
                        "Can not read files without role 'read'.",
                        ctx.clone()
                    ));
                }
                let path = Self::sanitized_absolute_path(ctx, &permissions, &arguments.path)?;

                Self::read_file(ctx, &path, arguments.start, arguments.length)
            }
            "list" => {
                if !permissions.roles.contains(&"list".to_string()) {
                    return Err(app_error!(
                        Unauthorized,
                        "skill_failed",
                        "Can not list directories without role 'list'.",
                        ctx.clone()
                    ));
                }
                let path = Self::sanitized_absolute_path(ctx, &permissions, &arguments.path)?;

                let mut items = Vec::new();
                Self::list_dir(ctx, &path, arguments.recursive, &mut items)?;

                let chroot_prefix = ctx.chroot.trim_end_matches('/');
                let mut items_sanitized = Vec::new();
                for item in items {
                    if Self::has_access_to_path(ctx, &permissions, &item) {
                        items_sanitized.push(item.trim_start_matches(&chroot_prefix).to_string());
                    }
                }

                Ok(items_sanitized.join("\n"))
            }
            "write" => {
                if !permissions.roles.contains(&"write".to_string()) {
                    return Err(app_error!(
                        Unauthorized,
                        "skill_failed",
                        "Can not write files without role 'write'.",
                        ctx.clone()
                    ));
                }

                let Some(content) = arguments.content else {
                    return Err(app_error!(
                        Validation,
                        "skill_failed",
                        "Missing 'content' argument for write action.",
                        ctx.clone()
                    ));
                };

                let parent = std::path::Path::new(&arguments.path)
                    .parent()
                    .ok_or_else(|| {
                        app_error!(
                            Validation,
                            "skill_failed",
                            &format!("Invalid path '{}'", arguments.path),
                            ctx.clone()
                        )
                    })?
                    .to_string_lossy()
                    .to_string();
                let file_name = &std::path::Path::new(&arguments.path)
                    .file_name()
                    .ok_or_else(|| {
                        app_error!(
                            Validation,
                            "skill_failed",
                            &format!("Invalid path '{}'", arguments.path),
                            ctx.clone()
                        )
                    })?
                    .to_string_lossy()
                    .to_string();

                let parent_path = Self::sanitized_absolute_path(ctx, &permissions, &parent)?;
                let path_str = parent_path.to_string_lossy().to_string() + "/" + &file_name;
                let path = std::path::Path::new(&path_str);

                Self::write_file(
                    ctx,
                    &path,
                    arguments.start,
                    arguments.length,
                    arguments.append,
                    &content,
                )?;

                Ok("Action completed successfully √".to_string())
            }
            "delete" => {
                if !permissions.roles.contains(&"delete".to_string()) {
                    return Err(app_error!(
                        Unauthorized,
                        "skill_failed",
                        "Can not delete files without role 'delete'.",
                        ctx.clone()
                    ));
                }
                let path = Self::sanitized_absolute_path(ctx, &permissions, &arguments.path)?;

                if path.is_dir() {
                    std::fs::remove_dir_all(&path).map_err(|e| {
                        app_error!(
                            Internal,
                            "skill_failed",
                            &format!("Failed to delete directory '{}': {}", path.display(), e),
                            ctx.clone()
                        )
                    })?;
                } else {
                    std::fs::remove_file(&path).map_err(|e| {
                        app_error!(
                            Internal,
                            "skill_failed",
                            &format!("Failed to delete file '{}': {}", path.display(), e),
                            ctx.clone()
                        )
                    })?;
                }
                Ok("Action completed successfully √".to_string())
            }
            "mkdir" => {
                if !permissions.roles.contains(&"mkdir".to_string()) {
                    return Err(app_error!(
                        Unauthorized,
                        "skill_failed",
                        "Can not create directories without role 'mkdir'.",
                        ctx.clone()
                    ));
                }

                let parent = std::path::Path::new(&arguments.path)
                    .parent()
                    .ok_or_else(|| {
                        app_error!(
                            Validation,
                            "skill_failed",
                            &format!("Invalid path '{}'", arguments.path),
                            ctx.clone()
                        )
                    })?
                    .to_string_lossy()
                    .to_string();
                let dir_name = &std::path::Path::new(&arguments.path)
                    .file_name()
                    .ok_or_else(|| {
                        app_error!(
                            Validation,
                            "skill_failed",
                            &format!("Invalid path '{}'", arguments.path),
                            ctx.clone()
                        )
                    })?
                    .to_string_lossy()
                    .to_string();

                let parent_path = Self::sanitized_absolute_path(ctx, &permissions, &parent)?;
                let path_str = parent_path.to_string_lossy().to_string() + "/" + &dir_name;
                let path = std::path::Path::new(&path_str);

                Self::create_dir(ctx, &path)?;
                Ok("Action completed successfully √".to_string())
            }

            other => Err(app_error!(
                Validation,
                "skill_failed",
                &format!("Unsupported action '{}'", other),
                ctx.clone()
            )),
        }
    }
}
