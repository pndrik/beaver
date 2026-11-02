// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::{configuration::ConfigurationMap, context::ContextStandard};
use app_domains::core::models::AppContext;

use super::{file_system, inference_providers, logging};

fn context(provider: &str) -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(
            [("logger.provider", provider)]
                .into_iter()
                .collect::<ConfigurationMap>(),
        ),
    )))
}

#[tokio::test]
async fn resolves_the_stdout_logger() {
    assert!(logging(&context("stdout")).await.is_ok());
}

#[tokio::test]
async fn rejects_unknown_logger_providers() {
    let err = logging(&context("syslog")).await.err().unwrap();
    assert_eq!(err.translation_key, "bootstrap_failed");
    assert!(err.internal_message.contains("syslog"));
}

fn file_system_context(values: &[(&str, &str)]) -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(values.iter().copied().collect::<ConfigurationMap>()),
    )))
}

#[tokio::test]
async fn resolves_file_systems() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().to_string_lossy().to_string();
    let disk = [
        ("file_system.provider", "disk"),
        ("file_system.disk.root", root.as_str()),
    ];
    assert!(file_system(&file_system_context(&disk)).await.is_ok());
    let memory = [("file_system.provider", "memory")];
    assert!(file_system(&file_system_context(&memory)).await.is_ok());
}

#[tokio::test]
async fn rejects_unknown_file_systems_and_missing_roots() {
    let s3 = [("file_system.provider", "s3")];
    let err = file_system(&file_system_context(&s3)).await.err().unwrap();
    assert_eq!(err.translation_key, "bootstrap_failed");

    let missing = [
        ("file_system.provider", "disk"),
        ("file_system.disk.root", "/does/not/exist"),
    ];
    assert!(file_system(&file_system_context(&missing)).await.is_err());
}

fn providers_context(enabled: &[(&str, bool)]) -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(
            enabled
                .iter()
                .map(|(provider, enabled)| {
                    (format!("inference.providers.{provider}.enabled"), *enabled)
                })
                .collect::<ConfigurationMap>(),
        ),
    )))
}

#[tokio::test]
async fn resolves_the_enabled_openai_provider() {
    let ctx = providers_context(&[("anthropic", false), ("openai", true), ("zen", false)]);
    assert_eq!(inference_providers(&ctx).await.unwrap().len(), 1);
}

#[tokio::test]
async fn resolves_no_disabled_or_unconfigured_providers() {
    let disabled = providers_context(&[("anthropic", false), ("openai", false), ("zen", false)]);
    assert!(inference_providers(&disabled).await.unwrap().is_empty());

    let missing = providers_context(&[]);
    assert!(inference_providers(&missing).await.unwrap().is_empty());
}

#[tokio::test]
async fn resolves_every_enabled_provider() {
    let ctx = providers_context(&[("anthropic", true), ("openai", true), ("zen", true)]);
    assert_eq!(inference_providers(&ctx).await.unwrap().len(), 3);
}
