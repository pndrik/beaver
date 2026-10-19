// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::{Regex, RegexBuilder};

use crate::{
    app_error,
    core::models::{AppContext, AppError},
};

const REGEX_SIZE_LIMIT: usize = 1024 * 1024;

pub(crate) fn regex(ctx: &AppContext, pattern: &str) -> Result<Regex, AppError> {
    RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .build()
        .map_err(|e| {
            app_error!(
                Validation,
                "regex_invalid",
                &format!("invalid regex '{}': {}", pattern, e),
                ctx.clone()
            )
        })
}
