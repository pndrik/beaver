// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::RegexBuilder;
use serde::{Deserialize, Serialize};

use super::invalid;
use crate::{
    app_error,
    core::models::{AppContext, AppError},
};

const REGEX_SIZE_LIMIT: usize = 1024 * 1024;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum Condition {
    All {
        all: Vec<Condition>,
    },
    Any {
        any: Vec<Condition>,
    },
    Compare {
        left: String,
        operator: Operator,
        right: String,
    },
}

impl Condition {
    pub fn validate(&self, ctx: &AppContext) -> Result<(), AppError> {
        match self {
            Condition::All { all } => validate_list(ctx, "all", all),
            Condition::Any { any } => validate_list(ctx, "any", any),
            Condition::Compare { .. } => Ok(()),
        }
    }
}

/// Rejects an empty list and validates every item.
pub(super) fn validate_list(
    ctx: &AppContext,
    kind: &str,
    conditions: &[Condition],
) -> Result<(), AppError> {
    if conditions.is_empty() {
        return Err(invalid(ctx, &format!("{} is empty", kind)));
    }
    conditions
        .iter()
        .try_for_each(|condition| condition.validate(ctx))
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    #[serde(rename = "==")]
    Eq,
    #[serde(rename = "!=")]
    Ne,
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = "<=")]
    Le,
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = ">=")]
    Ge,
    #[serde(rename = "~=")]
    Match,
}

impl Operator {
    /// Fails with `flow_condition_invalid` if a numeric side is not a number
    /// or the `~=` pattern is not a valid regex.
    pub fn compare(&self, ctx: &AppContext, left: &str, right: &str) -> Result<bool, AppError> {
        Ok(match self {
            Operator::Eq => left == right,
            Operator::Ne => left != right,
            Operator::Lt => number(ctx, left)? < number(ctx, right)?,
            Operator::Le => number(ctx, left)? <= number(ctx, right)?,
            Operator::Gt => number(ctx, left)? > number(ctx, right)?,
            Operator::Ge => number(ctx, left)? >= number(ctx, right)?,
            Operator::Match => RegexBuilder::new(right)
                .size_limit(REGEX_SIZE_LIMIT)
                .build()
                .map_err(|e| condition_invalid(ctx, &format!("invalid regex '{}': {}", right, e)))?
                .is_match(left),
        })
    }
}

fn number(ctx: &AppContext, value: &str) -> Result<f64, AppError> {
    value
        .trim()
        .parse()
        .map_err(|_| condition_invalid(ctx, &format!("'{}' is not a number", value)))
}

fn condition_invalid(ctx: &AppContext, message: &str) -> AppError {
    app_error!(
        Validation,
        "flow_condition_invalid",
        &format!("Condition failed: {}", message),
        ctx.clone()
    )
}
