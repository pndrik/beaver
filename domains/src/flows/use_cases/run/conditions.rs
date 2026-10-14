// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{future::Future, pin::Pin};

use serde_json::Value;

use crate::{
    core::{
        models::{AppContext, AppError},
        traits::TemplateEngine,
    },
    flows::models::Condition,
};

pub(super) fn any<'a>(
    engine: &'a (dyn TemplateEngine + Send + Sync),
    ctx: &'a AppContext,
    conditions: &'a [Condition],
    values: &'a Value,
) -> Pin<Box<dyn Future<Output = Result<bool, AppError>> + Send + 'a>> {
    Box::pin(async move {
        for condition in conditions {
            if eval(engine, ctx, condition, values).await? {
                return Ok(true);
            }
        }
        Ok(false)
    })
}

fn eval<'a>(
    engine: &'a (dyn TemplateEngine + Send + Sync),
    ctx: &'a AppContext,
    condition: &'a Condition,
    values: &'a Value,
) -> Pin<Box<dyn Future<Output = Result<bool, AppError>> + Send + 'a>> {
    Box::pin(async move {
        match condition {
            Condition::Any { any: conditions } => any(engine, ctx, conditions, values).await,
            Condition::All { all } => {
                for condition in all {
                    if !eval(engine, ctx, condition, values).await? {
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
