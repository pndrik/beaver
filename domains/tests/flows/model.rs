// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::json;

use app_domains::flows::models::{Action, Chroot, Definition};

use crate::support;

const EXAMPLE: &str = include_str!("../../../examples/flows/pr_review_flow.yaml");

fn parse(yaml: &str) -> Definition {
    yaml_serde::from_str(yaml).unwrap()
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
    assert_eq!(flow.metadata.name, "pr_review_flow");
    assert_eq!(
        flow.spec.triggers.webhook.as_ref().unwrap().name,
        "pr_review_flow"
    );
    assert_eq!(flow.spec.steps.len(), 2);

    let clone = &flow.spec.steps[0];
    assert_eq!(
        clone.context.chroot,
        Some(Chroot {
            path: "runs/{{ Ctx.trace_id }}".to_string(),
            create_if_missing: true,
            delete: true,
        })
    );
    let Action::Tool(tool) = &clone.action else {
        panic!("expected tool action");
    };
    assert_eq!(tool.tool, "git_clone");
    assert_eq!(tool.scopes, vec!["*"]);
    assert_eq!(tool.arguments["path"], json!("/repo"));

    let review = &flow.spec.steps[1];
    assert!(review.context.chroot.is_none());
    let Action::Agent(agent) = &review.action else {
        panic!("expected agent action");
    };
    assert_eq!(agent.agent, "coder");

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

const STEP_AGENT: &str = r#"
    - name: NAME
      context: { name: CONTEXT }
      action: { type: agent, arguments: { agent: AGENT, prompt: "hi" } }"#;

fn flow_with_steps(steps: &[(&str, &str, &str)]) -> String {
    let mut yaml = String::from("metadata: { name: f }\nspec:\n  steps:");
    for (name, context, agent) in steps {
        yaml.push_str(
            &STEP_AGENT
                .replace("NAME", name)
                .replace("CONTEXT", context)
                .replace("AGENT", agent),
        );
    }
    yaml
}

#[test]
fn rejects_empty_name() {
    let yaml = flow_with_steps(&[("a", "c", "x")]).replace("name: f", "name: \"\"");
    assert!(validation_error(&yaml).contains("metadata.name"));
}

#[test]
fn rejects_empty_steps() {
    assert!(validation_error("metadata: { name: f }\nspec: { steps: [] }").contains("empty"));
}

#[test]
fn rejects_duplicate_step_names() {
    let yaml = flow_with_steps(&[("a", "c", "x"), ("a", "c", "x")]);
    assert!(validation_error(&yaml).contains("more than once"));
}

#[test]
fn rejects_non_identifier_step_names() {
    for name in ["1a", "a-b", "a.b", "\"\""] {
        let yaml = flow_with_steps(&[(name, "c", "x")]);
        assert!(validation_error(&yaml).contains("must match"), "{}", name);
    }
}

#[test]
fn rejects_context_shared_by_different_agents() {
    let yaml = flow_with_steps(&[("a", "c", "x"), ("b", "c", "y")]);
    assert!(validation_error(&yaml).contains("context 'c'"));
}

#[test]
fn accepts_context_shared_by_same_agent() {
    let yaml = flow_with_steps(&[("a", "c", "x"), ("b", "c", "x"), ("_c2", "d", "y")]);
    let (ctx, _) = support::temp_context();
    parse(&yaml).validate(&ctx).unwrap();
}
