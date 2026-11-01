// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value};
use std::fmt::Debug;
use tracing::{
    Span,
    field::{Field, Visit},
};
use tracing_subscriber::registry::{LookupSpan, Registry, Scope};

use app_domains::core::models::AppContext;

const MESSAGE: &str = "message";
const RESERVED: [&str; 4] = ["timestamp", "level", "message", "ctx"];

struct Ctx(Map<String, Value>);

pub(super) fn context_span(ctx: &AppContext) -> Span {
    let span = tracing::error_span!(parent: None, "context");
    if let Ok(Value::Object(attributes)) = serde_json::to_value(ctx.log_attributes()) {
        span.with_subscriber(|(id, dispatch)| {
            if let Some(registry) = dispatch.downcast_ref::<Registry>()
                && let Some(span) = registry.span(id)
            {
                span.extensions_mut().insert(Ctx(attributes));
            }
        });
    }
    span
}

pub(super) fn scope_ctx<'a, R: LookupSpan<'a>>(
    scope: Option<Scope<'a, R>>,
) -> Option<Map<String, Value>> {
    scope?.find_map(|span| span.extensions().get::<Ctx>().map(|ctx| ctx.0.clone()))
}

#[derive(Debug, Default)]
pub(super) struct Fields {
    pub(super) message: String,
    pub(super) values: Map<String, Value>,
}

impl Fields {
    fn push(&mut self, field: &Field, value: Value) {
        match (field.name(), value) {
            (MESSAGE, Value::String(message)) => self.message = message,
            (name, _) if RESERVED.contains(&name) || name.starts_with("log.") => {}
            (name, value) => {
                self.values.insert(name.to_string(), value);
            }
        }
    }
}

impl Visit for Fields {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.push(field, Value::from(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.push(field, Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.push(field, Value::from(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.push(field, Value::from(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.push(field, Value::from(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.push(field, Value::from(format!("{value:?}")));
    }
}
