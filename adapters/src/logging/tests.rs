// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Value, json};
use std::sync::Arc;
use tracing::{Instrument, Level, info, info_span, warn};

use app_domains::core::{
    models::{AppContext, LogAttributes},
    traits::{Configuration, Context, Logging},
};

use super::Memory;
use crate::{configuration::ConfigurationMap, context::ContextStandard};

#[derive(Debug)]
struct Tenant(Arc<dyn Configuration + Send + Sync>);

impl Context for Tenant {
    fn trace_id(&self) -> &str {
        "t-1"
    }

    fn configuration(&self) -> &Arc<dyn Configuration + Send + Sync> {
        &self.0
    }

    fn template_values(&self) -> serde_json::Map<String, Value> {
        serde_json::Map::new()
    }

    fn log_attributes(&self) -> LogAttributes {
        LogAttributes::new(self.trace_id())
            .with("tenant", "acme")
            .with("seats", 3)
            .with("trace_id", "spoofed")
    }
}

fn standard() -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "abc".to_string(),
        Arc::new(ConfigurationMap::default()),
    )))
}

fn tenant() -> AppContext {
    AppContext::new(Arc::new(Tenant(Arc::new(ConfigurationMap::default()))))
}

fn memory() -> Memory {
    let memory = Memory::new();
    memory.init(&standard(), "trace").unwrap();
    memory
}

fn ctx_of(memory: &Memory, index: usize) -> Option<Value> {
    memory.events()[index].ctx.clone().map(Value::Object)
}

#[test]
fn records_typed_fields() {
    let memory = memory();
    info!(
        flow = "review",
        count = 3,
        ratio = 0.5,
        ok = true,
        "Flow started"
    );

    let event = &memory.events()[0];
    assert_eq!(event.message, "Flow started");
    assert_eq!(
        Value::Object(event.fields.clone()),
        json!({ "flow": "review", "count": 3, "ratio": 0.5, "ok": true })
    );
    assert_eq!(event.ctx, None);
}

#[test]
fn reserved_and_log_bridge_fields_are_dropped() {
    let memory = memory();
    warn!(level = "fake", ctx = "fake", log.file = "main.rs", "Real");

    assert!(memory.events()[0].fields.is_empty());
}

#[test]
fn context_span_adds_the_log_attributes() {
    let memory = memory();
    memory
        .context_span(&standard())
        .in_scope(|| info!("Inside"));

    assert_eq!(ctx_of(&memory, 0), Some(json!({ "trace_id": "abc" })));
}

#[test]
fn custom_attributes_keep_their_type_and_cannot_override_the_trace_id() {
    let memory = memory();
    memory.context_span(&tenant()).in_scope(|| info!("Inside"));

    assert_eq!(
        ctx_of(&memory, 0),
        Some(json!({ "trace_id": "t-1", "tenant": "acme", "seats": 3 }))
    );
}

#[test]
fn nested_spans_keep_the_nearest_ctx() {
    let memory = memory();
    memory.context_span(&standard()).in_scope(|| {
        info_span!("child").in_scope(|| info!("Nested"));
        memory
            .context_span(&tenant())
            .in_scope(|| info!("Own root"));
    });

    assert_eq!(ctx_of(&memory, 0), Some(json!({ "trace_id": "abc" })));
    assert_eq!(ctx_of(&memory, 1).unwrap()["trace_id"], "t-1");
}

#[tokio::test]
async fn ctx_survives_await() {
    let memory = memory();
    async {
        tokio::task::yield_now().await;
        info!(step = "a", "Step started");
    }
    .instrument(memory.context_span(&standard()))
    .await;

    assert!(memory.contains(Level::INFO, "Step started", &[("step", "a")]));
    assert_eq!(ctx_of(&memory, 0), Some(json!({ "trace_id": "abc" })));
}
