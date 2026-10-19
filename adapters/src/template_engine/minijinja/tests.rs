// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Value, json};
use std::sync::Arc;

use app_domains::core::{
    models::{AppContext, AppError},
    traits::TemplateEngine,
};

use super::MiniJinjaTemplateEngine;
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

async fn render(template: &str, values: Value) -> (Result<String, AppError>, Vec<String>) {
    let (ctx, logger) = context(&[]);
    let result = MiniJinjaTemplateEngine::new(&ctx)
        .render(&ctx, template, &values)
        .await;
    let warnings = logger.messages(RecordedLevel::Warn);
    (result, warnings)
}

#[tokio::test]
async fn renders_existing_values() {
    let (result, warnings) = render("Hi {{ Body.name }}", json!({"Body": {"name": "Ada"}})).await;
    assert_eq!(result.unwrap(), "Hi Ada");
    assert!(warnings.is_empty());
}

#[tokio::test]
async fn missing_leaf_renders_null_and_warns() {
    let (result, warnings) = render("[{{ Body.pr.title }}]", json!({"Body": {"pr": {}}})).await;
    assert_eq!(result.unwrap(), "[<NULL>]");
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("'Body.pr.title'"));
}

#[tokio::test]
async fn missing_intermediate_renders_null_and_warns_once() {
    let (result, warnings) = render("{{ Body.pr.title }} {{ Body.pr }}", json!({"Body": {}})).await;
    assert_eq!(result.unwrap(), "<NULL> <NULL>");
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("'Body.pr'"));
}

#[tokio::test]
async fn missing_root_renders_null_and_warns() {
    let (result, warnings) = render("{{ Variables.clone.output }}", json!({})).await;
    assert_eq!(result.unwrap(), "<NULL>");
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("'Variables.clone.output'"));
}

#[tokio::test]
async fn default_and_is_defined_see_undefined() {
    let (result, _) = render(
        "{{ Body.x | default('fallback') }}|{% if Body.x is defined %}yes{% else %}no{% endif %}|{% if Body.x %}t{% else %}f{% endif %}",
        json!({"Body": {}}),
    )
    .await;
    assert_eq!(result.unwrap(), "fallback|no|f");
}

#[tokio::test]
async fn does_not_html_escape() {
    let (result, _) = render(
        "{{ Body.s }}",
        json!({"Body": {"s": "<a href=\"x\">&</a>"}}),
    )
    .await;
    assert_eq!(result.unwrap(), "<a href=\"x\">&</a>");
}

#[tokio::test]
async fn keeps_trailing_newline() {
    let (result, _) = render("line\n", json!({})).await;
    assert_eq!(result.unwrap(), "line\n");
}

#[tokio::test]
async fn indent_filter_works() {
    let (result, _) = render(
        "x:\n  {{ Body.d | indent(2) }}",
        json!({"Body": {"d": "a\nb"}}),
    )
    .await;
    assert_eq!(result.unwrap(), "x:\n  a\n  b");
}

#[tokio::test]
async fn syntax_error_fails() {
    let (result, _) = render("{{ Body.x ", json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn renders_braces_next_to_expressions() {
    let (result, _) = render(r#"{"n": {{ Body.n }}}"#, json!({"Body": {"n": 37}})).await;
    assert_eq!(result.unwrap(), r#"{"n": 37}"#);
}

fn contexts() -> Value {
    json!({"Contexts": {
        "c": [
            {"type": "user", "content": "first"},
            {"type": "assistant", "content": "second"}
        ],
        "empty": []
    }})
}

#[tokio::test]
async fn last_returns_the_last_item() {
    let (result, warnings) = render("{{ Contexts.c.last().content }}", contexts()).await;
    assert_eq!(result.unwrap(), "second");
    assert!(warnings.is_empty(), "{:?}", warnings);
}

#[tokio::test]
async fn last_of_an_empty_list_is_undefined() {
    let (result, warnings) = render(
        "{{ Contexts.empty.last().content }}|{{ Contexts.empty.last() is defined }}",
        contexts(),
    )
    .await;
    assert_eq!(result.unwrap(), "<NULL>|False");
    assert!(warnings.is_empty(), "{:?}", warnings);
}

#[tokio::test]
async fn last_of_a_missing_context_warns_once() {
    let (result, warnings) = render(
        "{{ Contexts.missing.last().content }} {{ Contexts.missing.last().type }}",
        contexts(),
    )
    .await;
    assert_eq!(result.unwrap(), "<NULL> <NULL>");
    assert_eq!(warnings.len(), 1, "{:?}", warnings);
    assert!(warnings[0].contains("'Contexts.missing'"));
}

#[tokio::test]
async fn unknown_methods_still_fail() {
    let (result, _) = render("{{ Contexts.c.first() }}", contexts()).await;
    assert!(result.is_err());
}
