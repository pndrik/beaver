// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use super::{Arguments, File};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    skills::models::Call,
};

impl File {
    pub(super) fn skill_call_to_arguments(
        ctx: &AppContext,
        input: &Call,
    ) -> Result<Arguments, AppError> {
        let Some(action) = input.get_argument("action").and_then(|v| v.as_string()) else {
            return Err(app_error!(
                Validation,
                "skill_failed",
                "Missing required argument 'action'",
                ctx.clone()
            ));
        };

        let path = match input.get_argument("path").and_then(|v| v.as_string()) {
            Some(p) => p,
            None => {
                return Err(app_error!(
                    Validation,
                    "skill_failed",
                    "Missing required argument 'path'",
                    ctx.clone()
                ));
            }
        };

        let start = match input.get_argument("start").and_then(|v| v.as_int()) {
            Some(s) => {
                if s <= 0 {
                    return Err(app_error!(
                        Validation,
                        "skill_failed",
                        "'start' must be >= 1",
                        ctx.clone()
                    ));
                }
                Some(s)
            }
            None => None,
        };

        let length = match input.get_argument("length").and_then(|v| v.as_int()) {
            Some(l) => {
                if l <= 0 {
                    return Err(app_error!(
                        Validation,
                        "skill_failed",
                        "'length' must be >= 1",
                        ctx.clone()
                    ));
                }
                Some(l)
            }
            None => None,
        };

        let recursive = input
            .get_argument("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let append = input
            .get_argument("append")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let content = input
            .get_argument("content")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string());

        Ok(Arguments {
            action,
            path,
            start: start.map(|s| s as u64),
            length: length.map(|l| l as u64),
            recursive,
            append,
            content,
        })
    }
}
