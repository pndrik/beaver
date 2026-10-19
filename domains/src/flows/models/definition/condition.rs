// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{future::Future, pin::Pin};

use super::invalid;
use crate::{
    app_error,
    core::{
        models::{AppContext, AppError},
        traits::TemplateEngine,
    },
    flows::helpers::regex,
};

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

    /// Renders both sides of every comparison against `values`.
    pub(crate) fn eval<'a>(
        &'a self,
        ctx: &'a AppContext,
        engine: &'a (dyn TemplateEngine + Send + Sync),
        values: &'a Value,
    ) -> Pin<Box<dyn Future<Output = Result<bool, AppError>> + Send + 'a>> {
        Box::pin(async move {
            match self {
                Condition::Any { any } => any_of(ctx, engine, any, values).await,
                Condition::All { all } => {
                    for condition in all {
                        if !condition.eval(ctx, engine, values).await? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
                Condition::Compare {
                    left,
                    operator,
                    right,
                } => {
                    let left = engine.render(ctx, left, values).await?;
                    let right = engine.render(ctx, right, values).await?;
                    operator.compare(ctx, &left, &right)
                }
            }
        })
    }
}

/// True if any of `conditions` is true.
pub(crate) async fn any_of(
    ctx: &AppContext,
    engine: &(dyn TemplateEngine + Send + Sync),
    conditions: &[Condition],
    values: &Value,
) -> Result<bool, AppError> {
    for condition in conditions {
        if condition.eval(ctx, engine, values).await? {
            return Ok(true);
        }
    }
    Ok(false)
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
    /// and with `regex_invalid` if the `~=` pattern is not a valid regex.
    pub fn compare(&self, ctx: &AppContext, left: &str, right: &str) -> Result<bool, AppError> {
        Ok(match self {
            Operator::Eq => left == right,
            Operator::Ne => left != right,
            Operator::Lt => number(ctx, left)? < number(ctx, right)?,
            Operator::Le => number(ctx, left)? <= number(ctx, right)?,
            Operator::Gt => number(ctx, left)? > number(ctx, right)?,
            Operator::Ge => number(ctx, left)? >= number(ctx, right)?,
            Operator::Match => regex(ctx, right)?.is_match(left),
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
