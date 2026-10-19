// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Value, json};

use app_adapters::logger::RecordedLevel;
use app_domains::{
    core::models::ErrorKind,
    flows::models::{Definition, Hook},
};

use crate::support;

fn flow(name: &str, hook: &str, token: &str) -> Definition {
    yaml_serde::from_str(&format!(
        r#"
version: 1
metadata: {{ name: {name} }}
spec:
  contexts:
    c: {{ agent: coder }}
  triggers:
    webhook: {{ name: {hook}, token: "{token}" }}
  steps:
    - name: s
      context: c
      type: agent
      arguments: {{ prompt: p }}
"#
    ))
    .unwrap()
}

async fn resolve(flows: Vec<Definition>, name: &str, token: &str) -> Result<Hook, ErrorKind> {
    let (ctx, _) = support::temp_context();
    let fixture = support::fixture(&ctx, flows);
    fixture
        .flows
        .resolve_hook(&ctx, name, token, &json!({}), &json!({}))
        .await
        .map_err(|e| e.kind)
}

#[tokio::test]
async fn resolves_flow_with_valid_token() {
    let result = resolve(vec![flow("f", "hook", "secret")], "hook", "secret").await;
    assert!(matches!(result, Ok(hook) if hook.flow.metadata.name == "f"));
}

#[tokio::test]
async fn rejects_wrong_or_empty_tokens() {
    for token in ["wrong", "secre", "secret2", ""] {
        let result = resolve(vec![flow("f", "w", "secret")], "w", token).await;
        assert_eq!(result.unwrap_err(), ErrorKind::Unauthorized, "{}", token);
    }
}

#[tokio::test]
async fn rejects_configured_empty_token() {
    let result = resolve(vec![flow("f", "w", "")], "w", "").await;
    assert_eq!(result.unwrap_err(), ErrorKind::Unauthorized);
}

#[tokio::test]
async fn unknown_name_is_unauthorized() {
    let result = resolve(vec![flow("f", "w", "secret")], "other", "secret").await;
    assert_eq!(result.unwrap_err(), ErrorKind::Unauthorized);
}

fn mapped_flow() -> Definition {
    yaml_serde::from_str(
        r#"
version: 1
metadata: { name: mapped }
spec:
  arguments:
    title: { required: true }
    mode: {}
    n: { type: number }
  contexts:
    c: { agent: coder }
  triggers:
    webhook:
      name: w
      token: secret
      arguments:
        title: "{{ Body.pr.title }}"
        mode: "{{ Query.mode }}|{{ Flow.name }}|{{ Ctx.trace_id }}"
        n: "{{ Body.n }}"
  steps:
    - { name: s, context: c, type: agent, arguments: { prompt: "{{ Arguments.title }}|{{ Body.pr.title }}|{{ Query.mode }}" } }
"#,
    )
    .unwrap()
}

#[tokio::test]
async fn webhook_arguments_map_body_and_query() {
    let (ctx, logger) = support::temp_context();
    let fixture = support::fixture(&ctx, vec![mapped_flow()]);

    let Hook { flow, arguments } = fixture
        .flows
        .resolve_hook(
            &ctx,
            "w",
            "secret",
            &json!({"pr": {"title": "Fix bug"}, "n": 7}),
            &json!({"mode": "fast"}),
        )
        .await
        .unwrap();
    assert_eq!(
        Value::Object(arguments.clone()),
        json!({"title": "Fix bug", "mode": "fast|mapped|test-trace", "n": 7})
    );

    // Step templates only see the mapped arguments.
    fixture
        .flows
        .run(&ctx, &flow, &arguments, &fixture.inference, &fixture.tools)
        .await
        .unwrap();
    assert_eq!(fixture.scripted.calls()[0].prompt, "Fix bug|<NULL>|<NULL>");
    let warnings = logger.messages(RecordedLevel::Warn);
    assert!(
        warnings.iter().any(|w| w.contains("'Body.pr.title'")),
        "{:?}",
        warnings
    );
    assert!(
        warnings.iter().any(|w| w.contains("'Query.mode'")),
        "{:?}",
        warnings
    );
}

#[tokio::test]
async fn webhook_arguments_reject_invalid_values() {
    let (ctx, _) = support::temp_context();
    let fixture = support::fixture(&ctx, vec![mapped_flow()]);

    let err = fixture
        .flows
        .resolve_hook(
            &ctx,
            "w",
            "secret",
            &json!({"pr": {"title": "Fix bug"}, "n": "abc"}),
            &json!({}),
        )
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Validation);
    assert_eq!(err.translation_key, "flow_arguments_invalid");
    assert!(err.internal_message.contains("argument 'n'"));
}

#[tokio::test]
async fn flows_without_arguments_run_from_webhooks_and_flow_steps() {
    let (ctx, _) = support::temp_context();
    let plain = flow("plain", "w", "secret");
    assert!(plain.spec.arguments.is_empty());
    let caller: Definition = yaml_serde::from_str(
        r#"
version: 1
metadata: { name: caller }
spec:
  steps:
    - { name: call, type: flow, arguments: { flow: plain } }
"#,
    )
    .unwrap();
    let fixture = support::fixture(&ctx, vec![plain.clone()]);

    let Hook { arguments, .. } = fixture
        .flows
        .resolve_hook(
            &ctx,
            "w",
            "secret",
            &json!({"any": "payload"}),
            &json!({"q": "1"}),
        )
        .await
        .unwrap();
    assert!(arguments.is_empty());

    for flow in [&plain, &caller] {
        fixture
            .flows
            .run(&ctx, flow, &arguments, &fixture.inference, &fixture.tools)
            .await
            .unwrap();
    }
}
