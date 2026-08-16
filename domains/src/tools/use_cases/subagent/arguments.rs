// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    tools::models::Call,
};

pub(super) struct Arguments {
    pub(super) action: String,
    pub(super) name: Option<String>,
    pub(super) prompt: Option<String>,
}

pub(super) fn tool_call_to_arguments(
    ctx: &AppContext,
    input: &Call,
) -> Result<Arguments, AppError> {
    let Some(action) = input.get_argument("action").and_then(|v| v.as_string()) else {
        return Err(app_error!(
            Validation,
            "tool_failed",
            "Missing required argument 'action'",
            ctx.clone()
        ));
    };

    let name = input.get_argument("name").and_then(|v| v.as_string());
    let prompt = input.get_argument("prompt").and_then(|v| v.as_string());

    Ok(Arguments {
        action,
        name,
        prompt,
    })
}
