// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use chrono::{SecondsFormat, Utc};
use serde_json::Value;
use std::io::{self, Write};
use tracing::{Event, Span, Subscriber, level_filters::LevelFilter};
use tracing_subscriber::{
    layer::{Context, Layer, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
};

use app_domains::{
    app_error,
    core::{
        models::{AppContext, AppError},
        traits::Logging,
    },
};

use super::context::{Fields, context_span, scope_ctx};

#[derive(Debug, Default)]
pub struct Stdout;

impl Stdout {
    pub fn new() -> Self {
        Self
    }
}

impl Logging for Stdout {
    fn init(&self, ctx: &AppContext, level: &str) -> Result<(), AppError> {
        let failed =
            |message: String| app_error!(Internal, "bootstrap_failed", &message, ctx.clone());
        let level: LevelFilter = level
            .parse()
            .map_err(|_| failed(format!("Invalid log level: {level}")))?;

        tracing_subscriber::registry()
            .with(level)
            .with(JsonLines)
            .try_init()
            .map_err(|e| failed(format!("Failed to initialize logging: {e}")))
    }

    fn context_span(&self, ctx: &AppContext) -> Span {
        context_span(ctx)
    }
}

struct JsonLines;
impl<S> Layer<S> for JsonLines
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);

        let mut line = fields.values;
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true);
        line.insert("timestamp".to_string(), Value::from(timestamp));
        line.insert(
            "level".to_string(),
            Value::from(event.metadata().level().as_str()),
        );
        if !fields.message.is_empty() {
            line.insert("message".to_string(), Value::from(fields.message));
        }
        if let Some(attributes) = scope_ctx(ctx.event_scope(event)) {
            line.insert("ctx".to_string(), Value::Object(attributes));
        }

        let _ = writeln!(io::stdout().lock(), "{}", Value::Object(line));
    }
}
