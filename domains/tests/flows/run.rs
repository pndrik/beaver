// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value, json};
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

/// A validated flow. `arguments`, `contexts` and `steps` are indented by
/// four spaces.
fn definition(name: &str, arguments: &str, contexts: &str, steps: &str) -> Definition {
    let yaml = format!(
        "version: 1\nmetadata: {{ name: {} }}\nspec:\n  arguments:\n{}\n  contexts:\n{}\n  steps:\n{}",
        name, arguments, contexts, steps
    );
    let flow: Definition = yaml_serde::from_str(&yaml).unwrap();
    let (ctx, _) = support::temp_context();
    flow.validate(&ctx).unwrap();
    flow
}

/// The arguments of `flow`.
const ARGUMENTS: &str = r#"
    title: {}
    mode: {}
    n: { type: number }
    min: { type: float }"#;

fn flow(contexts: &str, steps: &str) -> Definition {
    definition("test_flow", ARGUMENTS, contexts, steps)
}

/// A single context `c` of agent `coder`.
const CODER: &str = "    c: { agent: coder }";

fn arguments(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

async fn run(contexts: &str, steps: &str) -> Run {
    run_with(contexts, steps, |_| {}).await
}

async fn run_with(contexts: &str, steps: &str, setup: impl FnOnce(&TempDir)) -> Run {
    execute(
        flow(contexts, steps),
        json!({"title": "Fix bug", "mode": "fast"}),
        vec![],
        setup,
    )
    .await
}

async fn run_args(steps: &str, input: Value) -> Run {
    execute(flow(CODER, steps), input, vec![], |_| {}).await
}

/// Runs `flow` with `input` as arguments. Flow steps can call `children`.
async fn execute(
    flow: Definition,
    input: Value,
    children: Vec<Definition>,
    setup: impl FnOnce(&TempDir),
) -> Run {
    let global = tempfile::tempdir().unwrap();
    setup(&global);
    let (ctx, logger) = support::context(&global.path().to_string_lossy());
    let Fixture {
        flows,
        inference,
        tools,
        scripted,
    } = support::fixture(&ctx, children);

    let result = flows
        .run(&ctx, &flow, &arguments(input), &inference, &tools)
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
async fn contexts_reach_later_prompts() {
    let run = run(
        r#"
    t: { agent: coder }
    c: { agent: coder }"#,
        r#"
    - name: tool_step
      context: t
      type: tool
      arguments: { tool: echo, arguments: { text: "{{ Arguments.title }}" } }
    - name: agent_step
      context: c
      type: agent
      arguments: { prompt: "{{ Contexts.t.last().content }}|{{ Arguments.mode }}|{{ Flow.name }}" }
    - name: last
      context: c
      type: agent
      arguments: { prompt: "{{ Contexts.c.last().content }}|{{ Contexts.c | length }}|{{ Contexts.c[0].type }}" }
"#,
    )
    .await;

    run.result.unwrap();
    let first = &run.calls[0].prompt;
    assert!(first.starts_with("Name: echo\n"), "{}", first);
    assert!(
        first.ends_with(&format!(
            "Result: \necho Fix bug @ {}|fast|test_flow",
            run.chroot
        )),
        "{}",
        first
    );
    assert_eq!(run.calls[1].prompt, format!("reply 1: {}|2|user", first));
    assert!(run.logger.messages(RecordedLevel::Warn).is_empty());
}

#[tokio::test]
async fn missing_template_field_does_not_fail_the_flow() {
    let run = run(
        CODER,
        r#"
    - { name: only, context: c, type: agent, arguments: { prompt: "title={{ Arguments.missing }}" } }
"#,
    )
    .await;

    run.result.unwrap();
    assert_eq!(run.calls[0].prompt, "title=<NULL>");
    assert!(
        run.logger
            .messages(RecordedLevel::Warn)
            .iter()
            .any(|w| w.contains("Arguments.missing"))
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
    - { name: agent_step, context: c, type: agent, arguments: { prompt: "{{ Contexts.c.last().content }}" } }
"#,
    )
    .await;

    run.result.unwrap();
    let chroot = format!("{}/work", run.chroot);
    assert_eq!(run.calls[0].history, ["tool", "user"]);
    assert_eq!(run.calls[0].chroot, chroot);
    assert!(
        run.calls[0]
            .prompt
            .ends_with(&format!("echo hi @ {}", chroot))
    );
}

#[tokio::test]
async fn steps_run_in_their_context_chroot() {
    let run = run(
        r#"    c: { agent: coder, chroot: { path: "runs/{{ Ctx.trace_id }}", create: true } }"#,
        r#"
    - { name: work, context: c, type: agent, arguments: { prompt: "x" } }
    - { name: same, context: c, type: agent, arguments: { prompt: "y" } }
"#,
    )
    .await;

    run.result.unwrap();
    let expected = format!("{}/runs/test-trace", run.chroot);
    assert_eq!(run.calls[0].chroot, expected);
    assert_eq!(run.calls[1].chroot, expected);
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
async fn contexts_are_created_before_the_first_step() {
    let run = run(
        r#"
    a: { agent: coder, chroot: { path: "runs/{{ Ctx.trace_id }}", create: true, delete: true } }
    later: { agent: coder, chroot: { path: "later", create: true } }"#,
        r#"
    - { name: first, context: a, type: tool, arguments: { tool: echo, arguments: { text: "hi" } } }
    - { name: second, context: later, type: tool, arguments: { tool: echo, arguments: { text: "{{ Contexts.later | length }}" } } }
"#,
    )
    .await;

    run.result.unwrap();
    // `later` exists before its first step, with an empty conversation.
    assert!(std::path::Path::new(&format!("{}/later", run.chroot)).is_dir());
    assert!(!std::path::Path::new(&format!("{}/runs/test-trace", run.chroot)).exists());
    assert!(run.logger.messages(RecordedLevel::Warn).is_empty());
}

#[tokio::test]
async fn invalid_context_fails_before_any_step_runs_and_cleans_up() {
    let run = run(
        r#"
    a: { agent: coder, chroot: { path: "runs/{{ Ctx.trace_id }}", create: true, delete: true } }
    b: { agent: missing }"#,
        r#"
    - { name: first, context: a, type: tool, arguments: { tool: echo, arguments: { text: "hi" } } }
    - { name: never, context: b, type: agent, arguments: { prompt: "x" } }
"#,
    )
    .await;

    let err = run.result.unwrap_err();
    assert!(
        err.internal_message
            .starts_with("Flow 'test_flow' context 'b' failed"),
        "{}",
        err.internal_message
    );
    assert!(!logged(
        &run.logger,
        RecordedLevel::Info,
        "step 'first' started"
    ));
    assert!(!std::path::Path::new(&format!("{}/runs/test-trace", run.chroot)).exists());
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
        .run(&ctx, &flow, &Map::new(), &inference, &tools)
        .await
        .unwrap_err();

    assert_eq!(err.translation_key, "flow_invalid");
    assert!(err.internal_message.contains("unknown context 'd'"));
    assert!(scripted.calls().is_empty());
    assert!(!global.path().join("work").exists());
}

fn logged(logger: &LoggerRecording, level: RecordedLevel, text: &str) -> bool {
    logger
        .messages(level)
        .iter()
        .any(|message| message.contains(text))
}

#[tokio::test]
async fn false_when_skips_the_step() {
    let run = run(
        CODER,
        r#"
    - name: a
      context: c
      type: agent
      when: [{ left: "{{ Arguments.title }}", operator: "==", right: "Other" }]
      arguments: { prompt: "x" }
    - { name: b, context: c, type: agent, arguments: { prompt: "{{ Contexts.c.last().content }}" } }
"#,
    )
    .await;

    run.result.unwrap();
    assert_eq!(run.calls.len(), 1);
    assert_eq!(run.calls[0].prompt, "<NULL>");
    assert!(logged(&run.logger, RecordedLevel::Info, "step 'a' skipped"));
    assert!(!logged(
        &run.logger,
        RecordedLevel::Info,
        "step 'a' started"
    ));
}

#[tokio::test]
async fn when_runs_the_step_if_any_item_is_true() {
    let run = run_args(
        r#"
    - name: a
      context: c
      type: agent
      when:
        - { left: "{{ Arguments.title }}", operator: "==", right: "Other" }
        - all:
          - { left: "{{ Arguments.title }}", operator: "~=", right: "(?i)^fix" }
          - any:
            - { left: "{{ Arguments.n }}", operator: "<", right: "1" }
            - { left: "{{ Arguments.n }}", operator: ">=", right: "{{ Arguments.min }}" }
      arguments: { prompt: "x" }
"#,
        json!({"title": "Fix bug", "n": 5, "min": "4.5"}),
    )
    .await;

    run.result.unwrap();
    assert_eq!(run.calls.len(), 1);
}

#[tokio::test]
async fn non_numeric_comparison_fails_the_flow() {
    let run = run(
        CODER,
        r#"
    - name: a
      context: c
      type: agent
      when: [{ left: "{{ Arguments.title }}", operator: ">", right: "1" }]
      arguments: { prompt: "x" }
"#,
    )
    .await;

    let err = run.result.unwrap_err();
    assert_eq!(err.translation_key, "flow_condition_invalid");
    assert!(err.internal_message.contains("step 'a' failed"));
    assert!(err.internal_message.contains("'Fix bug' is not a number"));
    assert!(run.calls.is_empty());
}

#[tokio::test]
async fn loop_runs_max_times_and_warns() {
    let run = run(
        CODER,
        r#"
    - { name: init, type: set, arguments: { i: { value: "0", type: number } } }
    - name: l
      type: loop
      arguments:
        steps:
          - { name: count, type: set, arguments: { i: { value: "{{ Variables.i + 1 }}", type: number } } }
          - { name: a, context: c, type: agent, arguments: { prompt: "x" } }
    - { name: after, context: c, type: agent, arguments: { prompt: "{{ Variables.i }}|{{ Contexts.c.last().content }}" } }
"#,
    )
    .await;

    run.result.unwrap();
    assert_eq!(run.calls.len(), 4);
    assert_eq!(run.calls[3].prompt, "3|reply 3: x");
    assert!(logged(
        &run.logger,
        RecordedLevel::Warn,
        "step 'l' stopped after max 3"
    ));
}

#[tokio::test]
async fn loop_stops_when_until_is_true() {
    let run = run(
        CODER,
        r#"
    - { name: init, type: set, arguments: { i: { value: "0", type: number } } }
    - name: l
      type: loop
      arguments:
        max: 5
        until: [{ left: "{{ Contexts.c.last().content }}", operator: "~=", right: "^reply 2:" }]
        steps:
          - { name: count, type: set, arguments: { i: { value: "{{ Variables.i + 1 }}", type: number } } }
          - { name: a, context: c, type: agent, arguments: { prompt: "x" } }
    - { name: after, context: c, type: agent, arguments: { prompt: "{{ Variables.i }}|{{ Contexts.c.last().content }}" } }
"#,
    )
    .await;

    run.result.unwrap();
    assert_eq!(run.calls.len(), 3);
    assert_eq!(run.calls[2].prompt, "2|reply 2: x");
    assert!(run.logger.messages(RecordedLevel::Warn).is_empty());
}

#[tokio::test]
async fn when_gates_the_loop_and_its_inner_steps() {
    let skipped = run(
        CODER,
        r#"
    - name: l
      type: loop
      when: [{ left: "{{ Arguments.title }}", operator: "==", right: "Other" }]
      arguments:
        steps:
          - { name: a, context: c, type: agent, arguments: { prompt: "x" } }
    - { name: after, context: c, type: agent, arguments: { prompt: "{{ Contexts.c.last().content }}" } }
"#,
    )
    .await;
    skipped.result.unwrap();
    assert_eq!(skipped.calls.len(), 1);
    assert_eq!(skipped.calls[0].prompt, "<NULL>");

    let inner = run(
        CODER,
        r#"
    - name: l
      type: loop
      arguments:
        steps:
          - { name: a, context: c, type: agent, arguments: { prompt: "x" } }
          - name: b
            context: c
            type: agent
            when: [{ left: "{{ Contexts.c.last().content }}", operator: "~=", right: "^reply 1:" }]
            arguments: { prompt: "y" }
"#,
    )
    .await;
    inner.result.unwrap();
    let prompts: Vec<&str> = inner
        .calls
        .iter()
        .map(|call| call.prompt.as_str())
        .collect();
    assert_eq!(prompts, ["x", "y", "x", "x"]);
}

#[tokio::test]
async fn failing_inner_step_fails_the_flow_and_cleans_up() {
    let run = run(
        r#"    c: { agent: coder, chroot: { path: "runs/{{ Ctx.trace_id }}", create: true, delete: true } }"#,
        r#"
    - name: l
      type: loop
      arguments:
        steps:
          - { name: ask, context: c, type: agent, arguments: { prompt: "x" } }
          - { name: broken, context: c, type: tool, arguments: { tool: fail } }
    - { name: never, context: c, type: agent, arguments: { prompt: "y" } }
"#,
    )
    .await;

    let err = run.result.unwrap_err();
    assert!(
        err.internal_message
            .starts_with("Flow 'test_flow' step 'broken' failed")
    );
    assert_eq!(run.calls.len(), 1);
    assert!(!std::path::Path::new(&format!("{}/runs/test-trace", run.chroot)).exists());
}

const CHECKED: &str = r#"
    name: { required: true, regex: "^[a-z]+$" }
    n: { type: number }
    f: { type: float }
    opt: {}"#;

const PRINT: &str = r#"
    - { name: a, context: c, type: agent, arguments: { prompt: "{{ Arguments.name }}|{{ Arguments.n * 2 }}|{{ Arguments.f }}|{{ Arguments.opt }}" } }
"#;

#[tokio::test]
async fn invalid_arguments_fail_before_any_step_runs() {
    for (input, expected) in [
        (
            json!({"name": "ok", "other": "x"}),
            "argument 'other' is not declared",
        ),
        (json!({}), "argument 'name' is required"),
        (
            json!({"name": "ok", "n": "abc"}),
            "argument 'n': \"abc\" is not a number",
        ),
        (
            json!({"name": "ok", "n": 1.5}),
            "argument 'n': 1.5 is not a number",
        ),
        (json!({"name": "ok", "f": "x"}), "argument 'f'"),
        (json!({"name": "Bad"}), "does not match '^[a-z]+$'"),
        (
            json!({"name": null}),
            "argument 'name': null is not a string",
        ),
    ] {
        let run = execute(
            definition("test_flow", CHECKED, CODER, PRINT),
            input.clone(),
            vec![],
            |_| {},
        )
        .await;
        let err = run.result.unwrap_err();
        assert_eq!(err.translation_key, "flow_arguments_invalid", "{}", input);
        assert!(
            err.internal_message.contains(expected),
            "{}: {}",
            input,
            err.internal_message
        );
        assert!(run.calls.is_empty());
    }
}

#[tokio::test]
async fn typed_arguments_are_rendered() {
    let run = execute(
        definition("test_flow", CHECKED, CODER, PRINT),
        json!({"name": "ok", "n": " 21 ", "f": "2.5"}),
        vec![],
        |_| {},
    )
    .await;

    run.result.unwrap();
    assert_eq!(run.calls[0].prompt, "ok|42|2.5|<NULL>");
    assert!(logged(&run.logger, RecordedLevel::Warn, "'Arguments.opt'"));
}

#[tokio::test]
async fn set_extracts_converts_and_falls_back() {
    let run = run(
        CODER,
        r#"
    - { name: ask, context: c, type: agent, arguments: { prompt: "The result was 1." } }
    - name: parse
      type: set
      arguments:
        result: { value: "{{ Contexts.c.last().content }}", type: number, extract: "result was (\\d+)" }
        missing: { value: "{{ Contexts.c.last().content }}", type: number, extract: "nothing (\\d+)" }
        fallback: { value: "abc", type: float, default: 1.5 }
        whole: { value: "id-42-x", extract: "\\d+" }
    - { name: use, context: c, type: agent, arguments: { prompt: "{{ Variables.result + 1 }}|{{ Variables.missing }}|{{ Variables.fallback }}|{{ Variables.whole }}" } }
"#,
    )
    .await;

    run.result.unwrap();
    assert_eq!(run.calls[1].prompt, "2|0|1.5|42");
    let warnings = run.logger.messages(RecordedLevel::Warn);
    assert_eq!(warnings.len(), 2, "{:?}", warnings);
    assert!(logged(
        &run.logger,
        RecordedLevel::Warn,
        "Flow 'test_flow' step 'parse': variable 'missing'"
    ));
    assert!(logged(
        &run.logger,
        RecordedLevel::Warn,
        "variable 'fallback': 'abc' is not a float, using default 1.5"
    ));
}

#[tokio::test]
async fn set_renders_all_variables_against_the_same_snapshot() {
    let run = run(
        CODER,
        r#"
    - { name: first, type: set, arguments: { a: { value: "1", type: number } } }
    - name: second
      type: set
      arguments:
        a: { value: "{{ Variables.a + 1 }}", type: number }
        b: { value: "{{ Variables.a }}" }
    - { name: use, context: c, type: agent, arguments: { prompt: "{{ Variables.a }}|{{ Variables.b }}" } }
"#,
    )
    .await;

    run.result.unwrap();
    assert_eq!(run.calls[0].prompt, "2|1");
}

#[tokio::test]
async fn variables_set_in_a_loop_drive_when_and_until() {
    let run = run(
        CODER,
        r#"
    - { name: init, type: set, arguments: { i: { value: "0", type: number } } }
    - name: l
      type: loop
      arguments:
        max: 5
        until: [{ left: "{{ Variables.i }}", operator: ">=", right: "3" }]
        steps:
          - { name: count, type: set, arguments: { i: { value: "{{ Variables.i + 1 }}", type: number } } }
          - name: a
            context: c
            type: agent
            when: [{ left: "{{ Variables.i }}", operator: "==", right: "2" }]
            arguments: { prompt: "i={{ Variables.i }}" }
    - { name: after, context: c, type: agent, arguments: { prompt: "{{ Variables.i }}" } }
"#,
    )
    .await;

    run.result.unwrap();
    let prompts: Vec<&str> = run.calls.iter().map(|call| call.prompt.as_str()).collect();
    assert_eq!(prompts, ["i=2", "3"]);
    assert!(run.logger.messages(RecordedLevel::Warn).is_empty());
}

fn child() -> Definition {
    definition(
        "child",
        "    x: { type: number, required: true }",
        CODER,
        r#"
    - { name: double, type: set, arguments: { doubled: { value: "{{ Arguments.x * 2 }}", type: number } } }
    - { name: a, context: c, type: agent, arguments: { prompt: "child {{ Arguments.x }}|{{ Contexts.p }}" } }
"#,
    )
}

#[tokio::test]
async fn flow_step_runs_the_child_and_exposes_its_variables() {
    let run = execute(
        flow(
            "    p: { agent: coder }",
            r#"
    - { name: before, context: p, type: agent, arguments: { prompt: "parent" } }
    - { name: call, type: flow, arguments: { flow: child, arguments: { x: "{{ Arguments.n }}" } } }
    - { name: after, context: p, type: agent, arguments: { prompt: "{{ Flows.call.doubled }}" } }
"#,
        ),
        json!({"n": 21}),
        vec![child()],
        |_| {},
    )
    .await;

    run.result.unwrap();
    let prompts: Vec<&str> = run.calls.iter().map(|call| call.prompt.as_str()).collect();
    // The child has its own contexts and doesn't see the parent's.
    assert_eq!(prompts, ["parent", "child 21|<NULL>", "42"]);
    assert_eq!(run.calls[1].history, ["user"]);
    assert_eq!(run.calls[2].history, ["user", "assistant", "user"]);
    assert!(logged(
        &run.logger,
        RecordedLevel::Info,
        "Flow 'child' finished"
    ));
}

#[tokio::test]
async fn flow_step_fails_on_invalid_child_arguments() {
    for mapping in ["{ y: \"1\" }", "{}", "{ x: abc }"] {
        let run = execute(
            flow(
                CODER,
                &format!(
                    r#"
    - {{ name: call, type: flow, arguments: {{ flow: child, arguments: {} }} }}
    - {{ name: never, context: c, type: agent, arguments: {{ prompt: "x" }} }}
"#,
                    mapping
                ),
            ),
            json!({}),
            vec![child()],
            |_| {},
        )
        .await;

        let err = run.result.unwrap_err();
        assert_eq!(err.translation_key, "flow_arguments_invalid", "{}", mapping);
        assert!(
            err.internal_message
                .starts_with("Flow 'test_flow' step 'call' failed: Flow 'child'")
        );
        assert!(run.calls.is_empty());
    }
}

#[tokio::test]
async fn flow_step_fails_with_the_child() {
    let missing = run(
        CODER,
        r#"
    - { name: call, type: flow, arguments: { flow: nope } }
    - { name: never, context: c, type: agent, arguments: { prompt: "x" } }
"#,
    )
    .await;
    assert_eq!(
        missing.result.unwrap_err().translation_key,
        "flow_not_found"
    );

    let failing = definition(
        "failing",
        "    {}",
        CODER,
        "    - { name: broken, context: c, type: tool, arguments: { tool: fail } }",
    );
    let run = execute(
        flow(
            CODER,
            r#"
    - { name: call, type: flow, arguments: { flow: failing } }
    - { name: never, context: c, type: agent, arguments: { prompt: "x" } }
"#,
        ),
        json!({}),
        vec![failing],
        |_| {},
    )
    .await;
    let err = run.result.unwrap_err();
    assert!(
        err.internal_message
            .contains("step 'call' failed: Flow 'failing' step 'broken' failed")
    );
    assert!(run.calls.is_empty());
}

#[tokio::test]
async fn recursive_flow_steps_stop_at_the_depth_limit() {
    let recursive = definition(
        "recursive",
        "    {}",
        CODER,
        r#"
    - { name: again, type: flow, arguments: { flow: recursive } }
    - { name: never, context: c, type: agent, arguments: { prompt: "x" } }
"#,
    );
    let run = execute(recursive.clone(), json!({}), vec![recursive], |_| {}).await;

    let err = run.result.unwrap_err();
    assert_eq!(err.translation_key, "flow_call_too_deep");
    let started = run
        .logger
        .messages(RecordedLevel::Info)
        .iter()
        .filter(|message| message.contains("Flow 'recursive' started"))
        .count();
    assert_eq!(started, 9);
    assert!(run.calls.is_empty());
}

#[tokio::test]
async fn pythagoras_example_calls_the_calculate_example() {
    let calculate: Definition =
        yaml_serde::from_str(include_str!("../../../examples/flows/calculate.yaml")).unwrap();
    let pythagoras: Definition =
        yaml_serde::from_str(include_str!("../../../examples/flows/pythagoras.yaml")).unwrap();

    // The scripted agent echoes the instruction, so `calculate` returns the
    // instruction's number: 3, 4, then the root of 3.0 + 4.0.
    let run = execute(
        pythagoras,
        json!({"a": "3", "b": 4}),
        vec![calculate],
        |_| {},
    )
    .await;

    run.result.unwrap();
    let prompts: Vec<&str> = run.calls.iter().map(|call| call.prompt.as_str()).collect();
    assert_eq!(
        prompts[..3],
        [
            "Square the number 3.0",
            "Square the number 4.0",
            "Take the square root of 7.0"
        ]
    );
    assert_eq!(run.calls[3].agent, "dispatcher");
    assert!(
        prompts[3]
            .contains("computed as 3.0 and 4.0,\ntheir sum as 7.0 and the hypotenuse as 7.0."),
        "{}",
        prompts[3]
    );
    assert!(run.logger.messages(RecordedLevel::Warn).is_empty());
}
