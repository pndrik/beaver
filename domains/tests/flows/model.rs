// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::json;

use app_domains::flows::models::{Action, Chroot, Definition};

use crate::support;

const EXAMPLE: &str = include_str!("../../../examples/flows/pr_review_flow.yaml");
const DOUBLE_CHECK: &str = include_str!("../../../examples/flows/double_check.yaml");

fn parse(yaml: &str) -> Definition {
    yaml_serde::from_str(yaml).unwrap()
}

fn parse_error(yaml: &str) -> String {
    yaml_serde::from_str::<Definition>(yaml)
        .unwrap_err()
        .to_string()
}

fn validation_error(yaml: &str) -> String {
    let (ctx, _) = support::temp_context();
    let err = parse(yaml).validate(&ctx).unwrap_err();
    assert_eq!(err.translation_key, "flow_invalid");
    err.internal_message
}

#[test]
fn parses_example() {
    let flow = parse(EXAMPLE);
    assert_eq!(flow.version, 1);
    assert_eq!(flow.metadata.name, "pr_review_flow");
    assert_eq!(
        flow.spec.triggers.webhook.as_ref().unwrap().name,
        "pr_review_flow"
    );
    assert_eq!(flow.spec.steps.len(), 2);

    let work = &flow.spec.contexts["work"];
    assert_eq!(work.agent, "coder");
    assert_eq!(
        work.chroot,
        Some(Chroot {
            path: "runs/{{ Ctx.trace_id }}".to_string(),
            create: true,
            delete: true,
        })
    );

    let clone = &flow.spec.steps[0];
    assert_eq!(clone.context, "work");
    let Action::Tool(tool) = &clone.action else {
        panic!("expected tool action");
    };
    assert_eq!(tool.tool, "git_clone");
    assert_eq!(tool.scopes, vec!["*"]);
    assert_eq!(tool.arguments["path"], json!("/repo"));

    let review = &flow.spec.steps[1];
    assert_eq!(review.context, "work");
    assert!(matches!(review.action, Action::Agent(_)));

    let (ctx, _) = support::temp_context();
    flow.validate(&ctx).unwrap();
}

#[test]
fn parses_double_check_example() {
    let flow = parse(DOUBLE_CHECK);
    assert_eq!(flow.spec.contexts.len(), 2);
    assert_eq!(flow.spec.contexts["calculator"].agent, "calculator");
    assert!(
        flow.spec.contexts["calculator"]
            .chroot
            .as_ref()
            .unwrap()
            .create
    );

    let (ctx, _) = support::temp_context();
    flow.validate(&ctx).unwrap();
}

#[test]
fn round_trips_through_yaml() {
    let flow = parse(EXAMPLE);
    let reparsed = parse(&yaml_serde::to_string(&flow).unwrap());
    assert_eq!(
        serde_json::to_value(&flow).unwrap(),
        serde_json::to_value(&reparsed).unwrap()
    );
}

/// A flow with one agent step per `(step, context)`. Every context named by a
/// step is declared with agent `x`.
fn flow_with_steps(steps: &[(&str, &str)]) -> String {
    let mut contexts: Vec<&str> = steps.iter().map(|(_, context)| *context).collect();
    contexts.sort();
    contexts.dedup();

    let mut yaml = String::from("version: 1\nmetadata: { name: f }\nspec:\n  contexts:");
    for context in contexts {
        yaml.push_str(&format!("\n    {}: {{ agent: x }}", context));
    }
    yaml.push_str("\n  steps:");
    for (name, context) in steps {
        yaml.push_str(&format!(
            "\n    - {{ name: {}, context: {}, type: agent, arguments: {{ prompt: hi }} }}",
            name, context
        ));
    }
    yaml
}

#[test]
fn accepts_shared_and_separate_contexts() {
    let yaml = flow_with_steps(&[("a", "c"), ("b", "c"), ("_c2", "d")]);
    let (ctx, _) = support::temp_context();
    parse(&yaml).validate(&ctx).unwrap();
}

#[test]
fn rejects_missing_version() {
    let yaml = flow_with_steps(&[("a", "c")]).replace("version: 1\n", "");
    assert!(parse_error(&yaml).contains("version"));
}

#[test]
fn rejects_unsupported_version() {
    let yaml = flow_with_steps(&[("a", "c")]).replace("version: 1", "version: 2");
    assert!(validation_error(&yaml).contains("unsupported version 2"));
}

#[test]
fn rejects_empty_name() {
    let yaml = flow_with_steps(&[("a", "c")]).replace("name: f", "name: \"\"");
    assert!(validation_error(&yaml).contains("metadata.name"));
}

#[test]
fn rejects_empty_steps() {
    let yaml = "version: 1\nmetadata: { name: f }\nspec: { steps: [] }";
    assert!(validation_error(yaml).contains("empty"));
}

#[test]
fn rejects_duplicate_step_names() {
    let yaml = flow_with_steps(&[("a", "c"), ("a", "c")]);
    assert!(validation_error(&yaml).contains("more than once"));
}

#[test]
fn rejects_non_identifier_step_names() {
    for name in ["1a", "a-b", "a.b", "\"\""] {
        let yaml = flow_with_steps(&[(name, "c")]);
        assert!(validation_error(&yaml).contains("must match"), "{}", name);
    }
}

#[test]
fn rejects_non_identifier_context_names() {
    let yaml = flow_with_steps(&[("a", "\"a-b\"")]);
    assert!(validation_error(&yaml).contains("context name 'a-b' must match"));
}

#[test]
fn rejects_unknown_step_context() {
    let yaml = flow_with_steps(&[("a", "c")]).replace("context: c", "context: d");
    let message = validation_error(&yaml);
    assert!(
        message.contains("step 'a' uses unknown context 'd'"),
        "{}",
        message
    );
}

#[test]
fn rejects_unused_context() {
    let yaml =
        flow_with_steps(&[("a", "c")]).replace("  steps:", "    unused: { agent: x }\n  steps:");
    assert!(validation_error(&yaml).contains("context 'unused' is not used by any step"));
}

#[test]
fn rejects_empty_context_agent() {
    let yaml = flow_with_steps(&[("a", "c")]).replace("agent: x", "agent: \"\"");
    assert!(validation_error(&yaml).contains("context 'c': agent is empty"));
}

#[test]
fn duplicate_context_key_replaces_the_earlier_one() {
    let yaml = flow_with_steps(&[("a", "c")]).replace(
        "    c: { agent: x }",
        "    c: { agent: x }\n    c: { agent: y }",
    );
    assert_eq!(parse(&yaml).spec.contexts["c"].agent, "y");
}

#[test]
fn rejects_unknown_fields() {
    let old_chroot = flow_with_steps(&[("a", "c")]).replace(
        "c: { agent: x }",
        "c: { agent: x, chroot: { path: p, create_if_missing: true } }",
    );
    assert!(parse_error(&old_chroot).contains("create_if_missing"));

    let old_agent = flow_with_steps(&[("a", "c")]).replace("prompt: hi", "agent: x, prompt: hi");
    assert!(parse_error(&old_agent).contains("agent"));
}

#[test]
fn rejects_the_old_action_wrapper() {
    let yaml = r#"
version: 1
metadata: { name: f }
spec:
  contexts:
    c: { agent: x }
  steps:
    - name: a
      context: c
      action: { type: agent, arguments: { prompt: hi } }
"#;
    parse_error(yaml);
}

#[test]
fn rejects_empty_webhook_token() {
    let yaml = flow_with_steps(&[("a", "c")]).replace(
        "  steps:",
        "  triggers:\n    webhook: { name: w, token: \"\" }\n  steps:",
    );
    assert!(validation_error(&yaml).contains("triggers.webhook.token"));
}
