// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
};

use super::File;

impl File {
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

    pub(super) fn write_file(
        ctx: &AppContext,
        path: &Path,
        start: Option<u64>,
        length: Option<u64>,
        append: bool,
        content: &str,
    ) -> Result<(), AppError> {
        let mut file = Self::open_file_for_writing(ctx, path, start, append)?;

        if append {
            use std::io::Write;
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

            return Self::replace_lines_in_file(ctx, &mut file, start, length, content);
        }

        // If no start is provided, overwrite the entire file
        use std::io::Write;
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
}
