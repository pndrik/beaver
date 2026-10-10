// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::json;
use tempfile::TempDir;

use app_adapters::{
    inference::ScriptedCall,
    logger::{LoggerRecording, RecordedLevel},
};
use app_domains::{
    core::models::{AppContext, AppError},
    inference::models::Flow,
};

use crate::support::{self, Fixture};

struct Run {
    result: Result<(), AppError>,
    calls: Vec<ScriptedCall>,
    logger: std::sync::Arc<LoggerRecording>,
    ctx: AppContext,
    // Keeps the global chroot alive until the assertions ran.
    _global: TempDir,
}

fn flow(steps: &str) -> Flow {
    let yaml = format!(
        "metadata: {{ name: test_flow }}\nspec:\n  steps:\n{}",
        steps
    );
    let flow: Flow = yaml_serde::from_str(&yaml).unwrap();
    let (ctx, _) = support::temp_context();
    flow.validate(&ctx).unwrap();
    flow
}

async fn run(steps: &str) -> Run {
    run_with(steps, |_| {}).await
}

async fn run_with(steps: &str, setup: impl FnOnce(&TempDir)) -> Run {
    let global = tempfile::tempdir().unwrap();
    setup(&global);
    let (ctx, logger) = support::context(&global.path().to_string_lossy());
    let Fixture {
        inference,
        tools,
        scripted,
    } = support::fixture(&ctx, vec![], vec![]);

    let result = inference
        .flow_run(
            &ctx,
            &flow(steps),
            &json!({"pr": {"title": "Fix bug"}}),
            &json!({"mode": "fast"}),
            &tools,
        )
        .await;
    let calls = scripted.calls();

    Run {
        result,
        calls,
        logger,
        ctx,
        _global: global,
    }
}

#[tokio::test]
async fn reused_context_keeps_the_conversation() {
    let run = run(r#"
    - name: first
      context: { name: c }
      action: { type: agent, arguments: { agent: coder, prompt: "one" } }
    - name: second
      context: { name: c }
      action: { type: agent, arguments: { agent: coder, prompt: "two" } }
    - name: other
      context: { name: d }
      action: { type: agent, arguments: { agent: coder, prompt: "three" } }
"#)
    .await;

    run.result.unwrap();
    assert_eq!(run.calls.len(), 3);
    assert!(run.calls.iter().all(|call| call.agent == "coder"));
    assert_eq!(run.calls[1].history, ["user", "assistant", "user"]);
    assert_eq!(run.calls[2].history, ["user"]);
}

#[tokio::test]
async fn step_outputs_reach_later_prompts() {
    let run = run(r#"
    - name: tool_step
      context: { name: t }
      action: { type: tool, arguments: { tool: echo, arguments: { text: "{{ Body.pr.title }}" } } }
    - name: agent_step
      context: { name: c }
      action: { type: agent, arguments: { agent: coder, prompt: "{{ Steps.tool_step.output }}|{{ Query.mode }}|{{ Flow.name }}" } }
    - name: last
      context: { name: c }
      action: { type: agent, arguments: { agent: coder, prompt: "{{ Steps.agent_step.output }}|{{ Steps.agent_step.context }}|{{ Steps.agent_step.chroot }}" } }
"#)
    .await;

    run.result.unwrap();
    let chroot = &run.ctx.chroot;
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
    let run = run(r#"
    - name: only
      context: { name: c }
      action: { type: agent, arguments: { agent: coder, prompt: "title={{ Body.pr.missing }}" } }
"#)
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
    let run = run(r#"
    - name: ask
      context: { name: c }
      action: { type: agent, arguments: { agent: coder, prompt: "one" } }
    - name: tool_step
      context: { name: c }
      action: { type: tool, arguments: { tool: echo, arguments: { text: "hi" } } }
    - name: again
      context: { name: c }
      action: { type: agent, arguments: { agent: coder, prompt: "two" } }
"#)
    .await;

    run.result.unwrap();
    assert_eq!(run.calls[1].history, ["user", "assistant", "tool", "user"]);
}

#[tokio::test]
async fn steps_run_in_their_context_chroot() {
    let run = run(r#"
    - name: work
      context: { name: c, chroot: { path: "runs/{{ Ctx.trace_id }}", create_if_missing: true } }
      action: { type: agent, arguments: { agent: coder, prompt: "{{ Steps.missing.output }}" } }
    - name: same
      context: { name: c }
      action: { type: agent, arguments: { agent: coder, prompt: "{{ Steps.work.chroot }}" } }
"#)
    .await;

    run.result.unwrap();
    let expected = format!("{}/runs/test-trace", run.ctx.chroot);
    assert_eq!(run.calls[0].chroot, expected);
    assert_eq!(run.calls[1].chroot, expected);
    assert_eq!(run.calls[1].prompt, "runs/test-trace");
    assert!(std::path::Path::new(&expected).is_dir());
}

#[tokio::test]
async fn failing_step_stops_the_flow_and_cleans_up() {
    let run = run(r#"
    - name: prepare
      context: { name: c, chroot: { path: "runs/{{ Ctx.trace_id }}", create_if_missing: true, delete: true } }
      action: { type: tool, arguments: { tool: echo, arguments: { text: "hi" } } }
    - name: broken
      context: { name: c }
      action: { type: tool, arguments: { tool: fail } }
    - name: never
      context: { name: c }
      action: { type: agent, arguments: { agent: coder, prompt: "x" } }
"#)
    .await;

    let err = run.result.unwrap_err();
    assert!(err.internal_message.contains("'test_flow'"));
    assert!(err.internal_message.contains("'broken'"));
    assert!(run.calls.is_empty());
    assert!(!std::path::Path::new(&format!("{}/runs/test-trace", run.ctx.chroot)).exists());
    assert!(std::path::Path::new(&run.ctx.chroot).is_dir());
}

#[tokio::test]
async fn unknown_agent_and_tool_fail_the_step() {
    let run_agent = run(r#"
    - name: a
      context: { name: c }
      action: { type: agent, arguments: { agent: missing, prompt: "x" } }
"#)
    .await;
    assert!(run_agent.result.is_err());

    let run_tool = run(r#"
    - name: a
      context: { name: c }
      action: { type: tool, arguments: { tool: nope } }
"#)
    .await;
    assert_eq!(
        run_tool.result.unwrap_err().translation_key,
        "tool_not_found"
    );
}

#[tokio::test]
async fn chroot_outside_the_global_chroot_is_rejected() {
    let outside = format!("escape-{}", std::process::id());
    let run = run(&format!(
        r#"
    - name: a
      context: {{ name: c, chroot: {{ path: "../{}", create_if_missing: true }} }}
      action: {{ type: agent, arguments: {{ agent: coder, prompt: "x" }} }}
"#,
        outside
    ))
    .await;

    assert_eq!(run.result.unwrap_err().translation_key, "access_violation");
    assert!(run.calls.is_empty());
    let parent = std::path::Path::new(&run.ctx.chroot).parent().unwrap();
    assert!(!parent.join(&outside).exists());

    let run = run_absolute("/").await;
    assert_eq!(run.result.unwrap_err().translation_key, "access_violation");
}

async fn run_absolute(path: &str) -> Run {
    run(&format!(
        r#"
    - name: a
      context: {{ name: c, chroot: {{ path: "{}" }} }}
      action: {{ type: agent, arguments: {{ agent: coder, prompt: "x" }} }}
"#,
        path
    ))
    .await
}

#[tokio::test]
async fn deleting_the_global_chroot_is_rejected() {
    let run = run(r#"
    - name: a
      context: { name: c, chroot: { path: ".", delete: true } }
      action: { type: agent, arguments: { agent: coder, prompt: "x" } }
"#)
    .await;

    assert_eq!(run.result.unwrap_err().translation_key, "access_violation");
    assert!(std::path::Path::new(&run.ctx.chroot).is_dir());
}

#[tokio::test]
async fn missing_chroot_without_create_fails() {
    let run = run(r#"
    - name: a
      context: { name: c, chroot: { path: "nope" } }
      action: { type: agent, arguments: { agent: coder, prompt: "x" } }
"#)
    .await;
    assert!(run.result.is_err());

    let run = run_with(
        r#"
    - name: a
      context: { name: c, chroot: { path: "exists" } }
      action: { type: agent, arguments: { agent: coder, prompt: "x" } }
"#,
        |global| std::fs::create_dir(global.path().join("exists")).unwrap(),
    )
    .await;
    run.result.unwrap();
    assert!(run.calls[0].chroot.ends_with("/exists"));
}

#[tokio::test]
async fn reused_context_with_a_different_chroot_warns_and_keeps_the_first() {
    let run = run(r#"
    - name: a
      context: { name: c, chroot: { path: "one", create_if_missing: true } }
      action: { type: agent, arguments: { agent: coder, prompt: "x" } }
    - name: b
      context: { name: c, chroot: { path: "two", create_if_missing: true } }
      action: { type: agent, arguments: { agent: coder, prompt: "y" } }
"#)
    .await;

    run.result.unwrap();
    assert!(run.calls[1].chroot.ends_with("/one"));
    assert!(
        run.logger
            .messages(RecordedLevel::Warn)
            .iter()
            .any(|w| w.contains("keeping the first one"))
    );
}

#[tokio::test]
async fn different_agent_on_a_reused_context_fails_before_any_step_runs() {
    let global = tempfile::tempdir().unwrap();
    let (ctx, _) = support::context(&global.path().to_string_lossy());
    let Fixture {
        inference,
        tools,
        scripted,
    } = support::fixture(&ctx, vec![], vec![]);
    // Built directly, without the provider's validation.
    let flow: Flow = yaml_serde::from_str(
        r#"
metadata: { name: test_flow }
spec:
  steps:
    - name: first
      context: { name: c, chroot: { path: "work", create_if_missing: true } }
      action: { type: agent, arguments: { agent: coder, prompt: "one" } }
    - name: second
      context: { name: c }
      action: { type: agent, arguments: { agent: reviewer, prompt: "two" } }
"#,
    )
    .unwrap();

    let err = inference
        .flow_run(&ctx, &flow, &json!({}), &json!({}), &tools)
        .await
        .unwrap_err();

    assert_eq!(err.translation_key, "flow_invalid");
    assert!(err.internal_message.contains("context 'c'"));
    assert!(scripted.calls().is_empty());
    assert!(!global.path().join("work").exists());
}
