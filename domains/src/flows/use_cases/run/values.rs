// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value, json};
use std::{future::Future, pin::Pin};

use crate::{
    core::{
        models::{AppContext, AppError},
        traits::TemplateEngine,
    },
    flows::models::Definition,
    inference::models::Message,
};

/// The values templates can read: `Body`, `Query`, `Ctx`, `Flow` and the
/// results of the steps that already ran under `Steps.<step>`.
pub(super) fn template_values(
    ctx: &AppContext,
    flow: &Definition,
    body: &Value,
    query: &Value,
    steps: &Map<String, Value>,
) -> Value {
    json!({
        "Body": body,
        "Query": query,
        "Ctx": ctx,
        "Flow": { "name": flow.metadata.name },
        "Steps": steps,
    })
}

pub(super) fn messages(messages: &[Message]) -> Value {
    Value::Array(
        messages
            .iter()
            .map(|message| {
                let agent_message = message.as_agent();
                json!({
                    "type": message.message_type().as_string(),
                    "agent": message.name(),
                    "content": message.content(),
                    "tool_calls": agent_message.map_or(&[][..], |m| &m.tool_calls),
                    "usage": agent_message.map(|m| m.usage),
                })
            })
            .collect(),
    )
}

/// Renders every string leaf of `arguments`, leaving other values untouched.
pub(super) fn render_arguments<'a>(
    engine: &'a (dyn TemplateEngine + Send + Sync),
    ctx: &'a AppContext,
    arguments: &'a Value,
    values: &'a Value,
) -> Pin<Box<dyn Future<Output = Result<Value, AppError>> + Send + 'a>> {
    Box::pin(async move {
        Ok(match arguments {
            Value::String(template) => Value::String(engine.render(ctx, template, values).await?),
            Value::Array(items) => {
                let mut rendered = Vec::with_capacity(items.len());
                for item in items {
                    rendered.push(render_arguments(engine, ctx, item, values).await?);
                }
                Value::Array(rendered)
            }
            Value::Object(fields) => {
                let mut rendered = Map::with_capacity(fields.len());
                for (key, value) in fields {
                    rendered.insert(
                        key.clone(),
                        render_arguments(engine, ctx, value, values).await?,
                    );
                }
                Value::Object(rendered)
            }
            other => other.clone(),
        })
    })
}
