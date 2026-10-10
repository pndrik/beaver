// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_adapters::logger::RecordedLevel;
use app_domains::{
    core::models::ErrorKind,
    inference::{
        models::{Flow, Webhook},
        use_cases::hook::HookTarget,
    },
};

use crate::support;

fn webhook(name: &str, token: &str) -> Webhook {
    yaml_serde::from_str(&format!(
        "metadata: {{ name: {name}, display_name: x, description: x, token: \"{token}\" }}\nhandler: {{ agent: a, prompt: p }}"
    ))
    .unwrap()
}

fn flow(name: &str, hook: &str, token: &str) -> Flow {
    yaml_serde::from_str(&format!(
        r#"
metadata: {{ name: {name} }}
spec:
  triggers:
    webhook: {{ name: {hook}, token: "{token}" }}
  steps:
    - name: s
      context: {{ name: c }}
      action: {{ type: agent, arguments: {{ agent: a, prompt: p }} }}
"#
    ))
    .unwrap()
}

async fn resolve(
    webhooks: Vec<Webhook>,
    flows: Vec<Flow>,
    name: &str,
    token: &str,
) -> (Result<HookTarget, ErrorKind>, Vec<String>) {
    let (ctx, logger) = support::temp_context();
    let fixture = support::fixture(&ctx, webhooks, flows);
    let result = fixture
        .inference
        .resolve_hook(&ctx, name, token)
        .await
        .map_err(|e| e.kind);
    let errors = logger.messages(RecordedLevel::Error);
    (result, errors)
}

#[tokio::test]
async fn resolves_webhook_with_valid_token() {
    let (result, _) = resolve(vec![webhook("w", "secret")], vec![], "w", "secret").await;
    assert!(matches!(result, Ok(HookTarget::Webhook(_))));
}

#[tokio::test]
async fn resolves_flow_with_valid_token() {
    let (result, _) = resolve(vec![], vec![flow("f", "hook", "secret")], "hook", "secret").await;
    assert!(matches!(result, Ok(HookTarget::Flow(f)) if f.metadata.name == "f"));
}

#[tokio::test]
async fn rejects_wrong_or_empty_tokens() {
    for token in ["wrong", "secre", "secret2", ""] {
        let (result, _) = resolve(vec![webhook("w", "secret")], vec![], "w", token).await;
        assert_eq!(result.unwrap_err(), ErrorKind::Unauthorized, "{}", token);

        let (result, _) = resolve(vec![], vec![flow("f", "w", "secret")], "w", token).await;
        assert_eq!(result.unwrap_err(), ErrorKind::Unauthorized, "{}", token);
    }
}

#[tokio::test]
async fn rejects_configured_empty_token() {
    let (result, _) = resolve(vec![webhook("w", "")], vec![], "w", "").await;
    assert_eq!(result.unwrap_err(), ErrorKind::Unauthorized);
}

#[tokio::test]
async fn unknown_name_is_unauthorized() {
    let (result, _) = resolve(vec![webhook("w", "secret")], vec![], "other", "secret").await;
    assert_eq!(result.unwrap_err(), ErrorKind::Unauthorized);
}

#[tokio::test]
async fn webhook_and_flow_with_the_same_name_run_neither() {
    let (result, errors) = resolve(
        vec![webhook("w", "secret")],
        vec![flow("f", "w", "secret")],
        "w",
        "secret",
    )
    .await;
    assert_eq!(result.unwrap_err(), ErrorKind::Unauthorized);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("both"));
}
