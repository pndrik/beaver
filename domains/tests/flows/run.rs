// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::json;
use tempfile::TempDir;

use app_adapters::{
    inference::ScriptedCall,
    logger::{LoggerRecording, RecordedLevel},
};
use app_domains::{core::models::AppError, flows::models::Definition};

use crate::support::{self, Fixture};

struct Run {
    result: Result<(), AppError>,
    calls: Vec<ScriptedCall>,
    logger: std::sync::Arc<LoggerRecording>,
    /// The canonical global chroot.
    chroot: String,
    // Keeps the global chroot alive until the assertions ran.
    _global: TempDir,
}

fn flow(contexts: &str, steps: &str) -> Definition {
    let yaml = format!(
        "version: 1\nmetadata: {{ name: test_flow }}\nspec:\n  contexts:\n{}\n  steps:\n{}",
        contexts, steps
    );
    let flow: Definition = yaml_serde::from_str(&yaml).unwrap();
    let (ctx, _) = support::temp_context();
    flow.validate(&ctx).unwrap();
    flow
}

/// A single context `c` of agent `coder`.
const CODER: &str = "    c: { agent: coder }";

async fn run(contexts: &str, steps: &str) -> Run {
    run_with(contexts, steps, |_| {}).await
}

async fn run_with(contexts: &str, steps: &str, setup: impl FnOnce(&TempDir)) -> Run {
    let global = tempfile::tempdir().unwrap();
    setup(&global);
    let (ctx, logger) = support::context(&global.path().to_string_lossy());
    let Fixture {
        flows,
        inference,
        tools,
        scripted,
    } = support::fixture(&ctx, vec![]);

    let result = flows
        .run(
            &ctx,
            &flow(contexts, steps),
            &json!({"pr": {"title": "Fix bug"}}),
            &json!({"mode": "fast"}),
            &inference,
            &tools,
        )
        .await;
    let calls = scripted.calls();
    let chroot = global
        .path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .to_string();

    Run {
        result,
        calls,
        logger,
        chroot,
        _global: global,
    }
}

#[tokio::test]
async fn reused_context_keeps_the_conversation() {
    let run = run(
        r#"
    c: { agent: coder }
    d: { agent: coder }"#,
        r#"
    - { name: first, context: c, type: agent, arguments: { prompt: "one" } }
    - { name: second, context: c, type: agent, arguments: { prompt: "two" } }
    - { name: other, context: d, type: agent, arguments: { prompt: "three" } }
"#,
    )
    .await;

    run.result.unwrap();
    assert_eq!(run.calls.len(), 3);
    assert!(run.calls.iter().all(|call| call.agent == "coder"));
    assert_eq!(run.calls[1].history, ["user", "assistant", "user"]);
    assert_eq!(run.calls[2].history, ["user"]);
}

#[tokio::test]
async fn step_outputs_reach_later_prompts() {
    let run = run(
        r#"
    t: { agent: coder }
    c: { agent: coder }"#,
        r#"
    - name: tool_step
      context: t
      type: tool
      arguments: { tool: echo, arguments: { text: "{{ Body.pr.title }}" } }
    - name: agent_step
      context: c
      type: agent
      arguments: { prompt: "{{ Steps.tool_step.output }}|{{ Query.mode }}|{{ Flow.name }}" }
    - name: last
      context: c
      type: agent
      arguments: { prompt: "{{ Steps.agent_step.output }}|{{ Steps.agent_step.context }}|{{ Steps.agent_step.chroot }}" }
"#,
    )
    .await;

    run.result.unwrap();
    let chroot = &run.chroot;
    assert_eq!(
        run.calls[0].prompt,
        format!("echo Fix bug @ {}|fast|test_flow", chroot)
    );
    assert_eq!(
        run.calls[1].prompt,
        format!("reply 1: echo Fix bug @ {}|fast|test_flow|c|.", chroot)
    );
}

#[tokio::test]
async fn missing_template_field_does_not_fail_the_flow() {
    let run = run(
        CODER,
        r#"
    - { name: only, context: c, type: agent, arguments: { prompt: "title={{ Body.pr.missing }}" } }
"#,
    )
    .await;

    run.result.unwrap();
    assert_eq!(run.calls[0].prompt, "title=<NULL>");
    assert!(
        run.logger
            .messages(RecordedLevel::Warn)
            .iter()
            .any(|w| w.contains("Body.pr.missing"))
    );
}

#[tokio::test]
async fn tool_result_is_added_to_an_existing_conversation() {
    let run = run(
        CODER,
        r#"
    - { name: ask, context: c, type: agent, arguments: { prompt: "one" } }
    - { name: tool_step, context: c, type: tool, arguments: { tool: echo, arguments: { text: "hi" } } }
    - { name: again, context: c, type: agent, arguments: { prompt: "two" } }
"#,
    )
    .await;

    run.result.unwrap();
    assert_eq!(run.calls[1].history, ["user", "assistant", "tool", "user"]);
}

#[tokio::test]
async fn tool_step_output_reaches_the_conversation_of_the_context() {
    let run = run(
        r#"    c: { agent: coder, chroot: { path: "work", create: true } }"#,
        r#"
    - { name: tool_step, context: c, type: tool, arguments: { tool: echo, arguments: { text: "hi" } } }
    - { name: agent_step, context: c, type: agent, arguments: { prompt: "{{ Steps.tool_step.output }}" } }
"#,
    )
    .await;

    run.result.unwrap();
    let chroot = format!("{}/work", run.chroot);
    assert_eq!(run.calls[0].history, ["tool", "user"]);
    assert_eq!(run.calls[0].chroot, chroot);
    assert_eq!(run.calls[0].prompt, format!("echo hi @ {}", chroot));
}

#[tokio::test]
async fn steps_run_in_their_context_chroot() {
    let run = run(
        r#"    c: { agent: coder, chroot: { path: "runs/{{ Ctx.trace_id }}", create: true } }"#,
        r#"
    - { name: work, context: c, type: agent, arguments: { prompt: "{{ Steps.missing.output }}" } }
    - { name: same, context: c, type: agent, arguments: { prompt: "{{ Steps.work.chroot }}" } }
"#,
    )
    .await;

    run.result.unwrap();
    let expected = format!("{}/runs/test-trace", run.chroot);
    assert_eq!(run.calls[0].chroot, expected);
    assert_eq!(run.calls[1].chroot, expected);
    assert_eq!(run.calls[1].prompt, "runs/test-trace");
    assert!(std::path::Path::new(&expected).is_dir());
}

#[tokio::test]
async fn failing_step_stops_the_flow_and_cleans_up() {
    let run = run(
        r#"    c: { agent: coder, chroot: { path: "runs/{{ Ctx.trace_id }}", create: true, delete: true } }"#,
        r#"
    - { name: prepare, context: c, type: tool, arguments: { tool: echo, arguments: { text: "hi" } } }
    - { name: broken, context: c, type: tool, arguments: { tool: fail } }
    - { name: never, context: c, type: agent, arguments: { prompt: "x" } }
"#,
    )
    .await;

    let err = run.result.unwrap_err();
    assert!(err.internal_message.contains("'test_flow'"));
    assert!(err.internal_message.contains("'broken'"));
    assert!(run.calls.is_empty());
    assert!(!std::path::Path::new(&format!("{}/runs/test-trace", run.chroot)).exists());
    assert!(std::path::Path::new(&run.chroot).is_dir());
}

#[tokio::test]
async fn contexts_are_created_when_their_first_step_runs() {
    let run = run(
        r#"
    c: { agent: coder }
    later: { agent: coder, chroot: { path: "later", create: true } }"#,
        r#"
    - { name: broken, context: c, type: tool, arguments: { tool: fail } }
    - { name: never, context: later, type: agent, arguments: { prompt: "x" } }
"#,
    )
    .await;

    assert!(run.result.is_err());
    assert!(!std::path::Path::new(&format!("{}/later", run.chroot)).exists());
}

#[tokio::test]
async fn unknown_agent_and_tool_fail_the_step() {
    let run_agent = run(
        "    c: { agent: missing }",
        r#"
    - { name: a, context: c, type: agent, arguments: { prompt: "x" } }
"#,
    )
    .await;
    assert!(run_agent.result.is_err());

    let run_tool = run(
        CODER,
        r#"
    - { name: a, context: c, type: tool, arguments: { tool: nope } }
    - { name: b, context: c, type: agent, arguments: { prompt: "x" } }
"#,
    )
    .await;
    assert_eq!(
        run_tool.result.unwrap_err().translation_key,
        "tool_not_found"
    );
}

const ONE_STEP: &str = r#"
    - { name: a, context: c, type: agent, arguments: { prompt: "x" } }
"#;

#[tokio::test]
async fn chroot_outside_the_global_chroot_is_rejected() {
    let outside = format!("escape-{}", std::process::id());
    let run = run(
        &format!(
            r#"    c: {{ agent: coder, chroot: {{ path: "../{}", create: true }} }}"#,
            outside
        ),
        ONE_STEP,
    )
    .await;

    assert_eq!(run.result.unwrap_err().translation_key, "access_violation");
    assert!(run.calls.is_empty());
    let parent = std::path::Path::new(&run.chroot).parent().unwrap();
    assert!(!parent.join(&outside).exists());

    let run = run_absolute("/").await;
    assert_eq!(run.result.unwrap_err().translation_key, "access_violation");
}

async fn run_absolute(path: &str) -> Run {
    run(
        &format!(
            r#"    c: {{ agent: coder, chroot: {{ path: "{}" }} }}"#,
            path
        ),
        ONE_STEP,
    )
    .await
}

#[tokio::test]
async fn deleting_the_global_chroot_is_rejected() {
    let run = run(
        r#"    c: { agent: coder, chroot: { path: ".", delete: true } }"#,
        ONE_STEP,
    )
    .await;

    assert_eq!(run.result.unwrap_err().translation_key, "access_violation");
    assert!(std::path::Path::new(&run.chroot).is_dir());
}

#[tokio::test]
async fn missing_chroot_without_create_fails() {
    let run = run(
        r#"    c: { agent: coder, chroot: { path: "nope" } }"#,
        ONE_STEP,
    )
    .await;
    assert!(run.result.is_err());

    let run = run_with(
        r#"    c: { agent: coder, chroot: { path: "exists" } }"#,
        ONE_STEP,
        |global| std::fs::create_dir(global.path().join("exists")).unwrap(),
    )
    .await;
    run.result.unwrap();
    assert!(run.calls[0].chroot.ends_with("/exists"));
}

#[tokio::test]
async fn unknown_context_fails_before_any_step_runs() {
    let global = tempfile::tempdir().unwrap();
    let (ctx, _) = support::context(&global.path().to_string_lossy());
    let Fixture {
        flows,
        inference,
        tools,
        scripted,
    } = support::fixture(&ctx, vec![]);
    // Built directly, without the provider's validation.
    let flow: Definition = yaml_serde::from_str(
        r#"
version: 1
metadata: { name: test_flow }
spec:
  contexts:
    c: { agent: coder, chroot: { path: "work", create: true } }
  steps:
    - { name: first, context: c, type: agent, arguments: { prompt: "one" } }
    - { name: second, context: d, type: agent, arguments: { prompt: "two" } }
"#,
    )
    .unwrap();

    let err = flows
        .run(&ctx, &flow, &json!({}), &json!({}), &inference, &tools)
        .await
        .unwrap_err();

    assert_eq!(err.translation_key, "flow_invalid");
    assert!(err.internal_message.contains("unknown context 'd'"));
    assert!(scripted.calls().is_empty());
    assert!(!global.path().join("work").exists());
}
