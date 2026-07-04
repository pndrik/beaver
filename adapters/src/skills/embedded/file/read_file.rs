// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
};

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
};

use super::File;

impl File {
    pub(super) fn read_file(
        ctx: &AppContext,
        path: &Path,
        start: Option<u64>,
        length: Option<u64>,
    ) -> Result<String, AppError> {
        if !path.is_file() {
            return Err(app_error!(
                Validation,
                "skill_failed",
                &format!("Path '{}' is not a file", path.display()),
                ctx.clone()
            ));
        }

        if start.is_none() && length.is_none() {
            return fs::read_to_string(path).map_err(|e| {
                app_error!(
                    Internal,
                    "skill_failed",
                    &format!("Failed to read file '{}': {}", path.display(), e),
                    ctx.clone()
                )
            });
        }

        let file = fs::File::open(path).map_err(|e| {
            app_error!(
                Internal,
                "skill_failed",
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
                    "skill_failed",
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
}
