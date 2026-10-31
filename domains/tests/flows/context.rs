// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value, json};
use tracing::Instrument;

use app_adapters::logging::Memory;
use app_domains::core::traits::Logging;
use app_domains::flows::models::{Definition, Hook};

use crate::support::{self, TENANT_TRACE_ID};

fn flow() -> Definition {
    yaml_serde::from_str(
        r#"
version: 1
metadata: { name: tenant }
spec:
  arguments:
    mapped: {}
  contexts:
    c: { agent: coder }
  triggers:
    webhook:
      name: w
      token: secret
      arguments:
        mapped: "{{ Ctx.tenant }}|{{ Ctx.trace_id }}"
  steps:
    - { name: s, context: c, type: agent, arguments: { prompt: "{{ Ctx.tenant }}|{{ Ctx.trace_id }}" } }
"#,
    )
    .unwrap()
}

/// Logs carry `log_attributes()`, not the template values, and a custom `trace_id` can't
/// override the real one.
fn assert_traced_by_log_attributes(logger: &Memory) {
    let events = logger.events();
    let expected = json!({ "trace_id": TENANT_TRACE_ID, "tenant": "acme" });
    assert!(!events.is_empty());
    assert!(
        events
            .iter()
            .all(|event| event.ctx.clone().map(Value::Object) == Some(expected.clone())),
        "{:?}",
        events
    );
}

#[tokio::test]
async fn flow_steps_see_only_the_template_values() {
    let (ctx, logger) = support::tenant_context();
    let fixture = support::fixture(&ctx, vec![]);

    fixture
        .flows
        .run(
            &ctx,
            &flow(),
            &Map::new(),
            &fixture.inference,
            &fixture.tools,
        )
        .instrument(logger.context_span(&ctx))
        .await
        .unwrap();

    assert_eq!(fixture.scripted.calls()[0].prompt, "acme|<NULL>");
    assert_traced_by_log_attributes(&logger);
}

#[tokio::test]
async fn webhook_arguments_see_only_the_template_values() {
    let (ctx, logger) = support::tenant_context();
    let fixture = support::fixture(&ctx, vec![flow()]);

    let Hook { arguments, .. } = fixture
        .flows
        .resolve_hook(&ctx, "w", "secret", &json!({}), &json!({}))
        .instrument(logger.context_span(&ctx))
        .await
        .unwrap();

    assert_eq!(Value::Object(arguments), json!({ "mapped": "acme|<NULL>" }));
    assert_traced_by_log_attributes(&logger);
}
