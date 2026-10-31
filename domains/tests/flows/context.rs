// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value, json};

use app_adapters::logger::LoggerRecording;
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

fn assert_traced_by_getter(logger: &LoggerRecording) {
    let logs = logger.logs();
    assert!(!logs.is_empty());
    assert!(
        logs.iter().all(|log| log.trace == TENANT_TRACE_ID),
        "{:?}",
        logs
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
        .await
        .unwrap();

    assert_eq!(fixture.scripted.calls()[0].prompt, "acme|<NULL>");
    assert_traced_by_getter(&logger);
}

#[tokio::test]
async fn webhook_arguments_see_only_the_template_values() {
    let (ctx, logger) = support::tenant_context();
    let fixture = support::fixture(&ctx, vec![flow()]);

    let Hook { arguments, .. } = fixture
        .flows
        .resolve_hook(&ctx, "w", "secret", &json!({}), &json!({}))
        .await
        .unwrap();

    assert_eq!(Value::Object(arguments), json!({ "mapped": "acme|<NULL>" }));
    assert_traced_by_getter(&logger);
}
