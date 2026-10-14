// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_domains::{core::models::ErrorKind, flows::models::Definition};

use crate::support;

fn flow(name: &str, hook: &str, token: &str) -> Definition {
    yaml_serde::from_str(&format!(
        r#"
version: 1
metadata: {{ name: {name} }}
spec:
  contexts:
    c: {{ agent: a }}
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

async fn resolve(flows: Vec<Definition>, name: &str, token: &str) -> Result<Definition, ErrorKind> {
    let (ctx, _) = support::temp_context();
    let fixture = support::fixture(&ctx, flows);
    fixture
        .flows
        .resolve_hook(&ctx, name, token)
        .await
        .map_err(|e| e.kind)
}

#[tokio::test]
async fn resolves_flow_with_valid_token() {
    let result = resolve(vec![flow("f", "hook", "secret")], "hook", "secret").await;
    assert!(matches!(result, Ok(f) if f.metadata.name == "f"));
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
