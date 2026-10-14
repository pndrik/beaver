// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{fs, sync::Arc};

use app_domains::{core::models::AppContext, flows::traits::FlowProvider};

use super::File;
use crate::{
    configuration::ConfigurationMap,
    logger::{LoggerRecording, RecordedLevel},
};

fn context(configuration: &[(&str, &str)]) -> (AppContext, Arc<LoggerRecording>) {
    let logger = Arc::new(LoggerRecording::new());
    let configuration: ConfigurationMap = configuration.iter().copied().collect();
    let ctx = AppContext::new("test".to_string(), Arc::new(configuration), logger.clone());
    (ctx, logger)
}

fn flow_yaml(name: &str, hook: &str, prompt: &str) -> String {
    format!(
        r#"
version: 1
metadata: {{ name: {name} }}
spec:
  contexts:
    c: {{ agent: a }}
  triggers:
    webhook: {{ name: {hook}, token: secret }}
  steps:
    - name: only
      context: c
      type: agent
      arguments: {{ prompt: "{prompt}" }}
"#
    )
}

fn prompt_of(flow: &app_domains::flows::models::Definition) -> String {
    match &flow.spec.steps[0].action {
        app_domains::flows::models::Action::Agent(arguments) => arguments.prompt.clone(),
        _ => panic!("expected agent step"),
    }
}

#[tokio::test]
async fn picks_up_added_changed_and_removed_files() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_string_lossy().to_string();
    let (ctx, _) = context(&[("flows.path", &dir_path)]);
    let provider = File::new(&ctx);

    assert!(
        provider
            .get_by_webhook(&ctx, "hook", "secret")
            .await
            .unwrap()
            .is_none()
    );

    let file = dir.path().join("my_flow.yaml");
    fs::write(&file, flow_yaml("my_flow", "hook", "first")).unwrap();
    let flow = provider
        .get_by_webhook(&ctx, "hook", "secret")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(prompt_of(&flow), "first");

    fs::write(&file, flow_yaml("my_flow", "hook", "second")).unwrap();
    let flow = provider
        .get_by_webhook(&ctx, "hook", "secret")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(prompt_of(&flow), "second");

    fs::remove_file(&file).unwrap();
    assert!(
        provider
            .get_by_webhook(&ctx, "hook", "secret")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn skips_invalid_flows_with_a_warning() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_string_lossy().to_string();
    let (ctx, logger) = context(&[("flows.path", &dir_path)]);
    let provider = File::new(&ctx);

    fs::write(dir.path().join("broken.yaml"), "metadata: [").unwrap();
    fs::write(
        dir.path().join("renamed.yaml"),
        flow_yaml("other_name", "hook", "x"),
    )
    .unwrap();
    fs::write(
        dir.path().join("good.yaml"),
        flow_yaml("good", "hook", "ok"),
    )
    .unwrap();

    let flow = provider
        .get_by_webhook(&ctx, "hook", "secret")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(flow.metadata.name, "good");

    let warnings = logger.messages(RecordedLevel::Warn);
    assert_eq!(warnings.len(), 2);
    assert!(warnings.iter().any(|w| w.contains("'broken'")));
    assert!(warnings.iter().any(|w| w.contains("'renamed'")));
}

#[tokio::test]
async fn returns_the_flow_only_for_its_token() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_string_lossy().to_string();
    let (ctx, _) = context(&[("flows.path", &dir_path)]);
    let provider = File::new(&ctx);
    fs::write(
        dir.path().join("my_flow.yaml"),
        flow_yaml("my_flow", "hook", "x"),
    )
    .unwrap();

    for token in ["wrong", "secre", "secret2", ""] {
        let flow = provider.get_by_webhook(&ctx, "hook", token).await.unwrap();
        assert!(flow.is_none(), "{}", token);
    }
    let flow = provider
        .get_by_webhook(&ctx, "other", "secret")
        .await
        .unwrap();
    assert!(flow.is_none());
    let flow = provider
        .get_by_webhook(&ctx, "hook", "secret")
        .await
        .unwrap();
    assert_eq!(flow.unwrap().metadata.name, "my_flow");
}

#[tokio::test]
async fn flow_without_a_token_accepts_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path().to_string_lossy().to_string();
    let (ctx, _) = context(&[("flows.path", &dir_path)]);
    let provider = File::new(&ctx);
    fs::write(
        dir.path().join("open.yaml"),
        flow_yaml("open", "hook", "x").replace("token: secret", "token: \"\""),
    )
    .unwrap();

    let flow = provider.get_by_webhook(&ctx, "hook", "").await.unwrap();
    assert!(flow.is_none());
}

#[tokio::test]
async fn no_flows_when_path_is_empty() {
    let (ctx, _) = context(&[("flows.path", "")]);
    let provider = File::new(&ctx);

    assert!(
        provider
            .get_by_webhook(&ctx, "hook", "secret")
            .await
            .unwrap()
            .is_none()
    );
    assert!(provider.list(&ctx).await.unwrap().is_empty());
}

#[tokio::test]
async fn example_flows_are_valid() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/flows");
    let (ctx, logger) = context(&[("flows.path", dir)]);

    let flows = File::new(&ctx).list(&ctx).await.unwrap();
    assert!(flows.iter().any(|f| f.metadata.name == "double_check"));
    assert!(logger.messages(RecordedLevel::Warn).is_empty());
}
