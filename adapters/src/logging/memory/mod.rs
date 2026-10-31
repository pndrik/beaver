// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value};
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};
use tracing::{Event, Level, Span, Subscriber, subscriber::DefaultGuard};
use tracing_subscriber::{
    layer::{Context, Layer, SubscriberExt},
    registry::LookupSpan,
};

use app_domains::core::{
    models::{AppContext, AppError},
    traits::Logging,
};

use super::context::{Fields, context_span, scope_ctx};

#[derive(Debug, Clone, PartialEq)]
pub struct RecordedEvent {
    pub level: Level,
    pub message: String,
    pub fields: Map<String, Value>,
    pub ctx: Option<Map<String, Value>>,
}

impl RecordedEvent {
    pub fn field(&self, name: &str) -> Option<String> {
        self.fields.get(name).map(|value| match value {
            Value::String(value) => value.clone(),
            other => other.to_string(),
        })
    }
}

#[derive(Debug, Default)]
pub struct Memory {
    events: Arc<Mutex<Vec<RecordedEvent>>>,
    guard: RefCell<Option<DefaultGuard>>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<RecordedEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn messages(&self, level: Level) -> Vec<String> {
        self.events()
            .into_iter()
            .filter(|event| event.level == level)
            .map(|event| event.message)
            .collect()
    }

    pub fn contains(&self, level: Level, message: &str, fields: &[(&str, &str)]) -> bool {
        self.events().iter().any(|event| {
            event.level == level
                && event.message == message
                && fields
                    .iter()
                    .all(|(name, value)| event.field(name).as_deref() == Some(*value))
        })
    }
}

impl Logging for Memory {
    fn init(&self, _ctx: &AppContext, _level: &str) -> Result<(), AppError> {
        let subscriber = tracing_subscriber::registry().with(Recorder(self.events.clone()));
        *self.guard.borrow_mut() = Some(tracing::subscriber::set_default(subscriber));
        Ok(())
    }

    fn context_span(&self, ctx: &AppContext) -> Span {
        context_span(ctx)
    }
}

struct Recorder(Arc<Mutex<Vec<RecordedEvent>>>);

impl<S> Layer<S> for Recorder
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut fields = Fields::default();
        event.record(&mut fields);

        self.0.lock().unwrap().push(RecordedEvent {
            level: *event.metadata().level(),
            message: fields.message,
            fields: fields.values,
            ctx: scope_ctx(ctx.event_scope(event)),
        });
    }
}
