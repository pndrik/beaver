// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

mod tool_denial;
pub use tool_denial::*;

mod tool_permission;
pub use tool_permission::*;

use serde::{Deserialize, Serialize};

use crate::{
    app_error,
    core::models::{AppContext, AppError, LiteralOrRegex},
};

pub(crate) fn invalid(ctx: &AppContext, message: &str) -> AppError {
    app_error!(Validation, "agent_invalid", message, ctx.clone())
}

/// Characters that make a name without a leading `^` look like a pattern.
const REGEX_SYNTAX: &[char] = &[
    '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '\\', '^',
];

/// Whether the rule name `pattern` matches `tool_name`: a name starting with
/// `^` is a regex that has to match the whole tool name (an invalid one
/// matches nothing), anything else has to be equal.
fn matches(pattern: &str, tool_name: &str) -> bool {
    LiteralOrRegex::parse(pattern).is_ok_and(|pattern| pattern.matches(tool_name))
}

fn check_name(ctx: &AppContext, name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(invalid(ctx, "name is empty"));
    }
    if name.starts_with('^') {
        LiteralOrRegex::parse(name)
            .map_err(|message| invalid(ctx, &format!("name '{name}' {message}")))?;
    } else if name.contains(REGEX_SYNTAX) {
        return Err(invalid(
            ctx,
            &format!("name '{name}' contains regex syntax; patterns must be written as ^...$"),
        ));
    }
    Ok(())
}

/// The tools an agent may call: deny rules first, then the first matching
/// allow rule.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ToolsConfiguration {
    #[serde(default)]
    pub allow: Vec<ToolPermission>,
    #[serde(default)]
    pub deny: Vec<ToolDenial>,
    /// Offers provider tools an optional `harness_redirect_to_file` argument.
    #[serde(default)]
    pub redirect_output: bool,
}

impl ToolsConfiguration {
    pub fn permission(&self, tool_name: &str) -> Option<&ToolPermission> {
        if self.is_denied(tool_name) {
            return None;
        }
        self.allow.iter().find(|rule| rule.matches(tool_name))
    }

    pub fn is_denied(&self, tool_name: &str) -> bool {
        self.deny.iter().any(|rule| rule.matches(tool_name))
    }

    pub fn validate(&self, ctx: &AppContext, location: &str) -> Result<(), AppError> {
        for (index, rule) in self.allow.iter().enumerate() {
            rule.validate(ctx).map_err(|err| {
                invalid(
                    ctx,
                    &format!("{location}.allow[{index}]: {}", err.internal_message),
                )
            })?;
        }
        for (index, rule) in self.deny.iter().enumerate() {
            rule.validate(ctx).map_err(|err| {
                invalid(
                    ctx,
                    &format!("{location}.deny[{index}]: {}", err.internal_message),
                )
            })?;
        }
        Ok(())
    }
}
