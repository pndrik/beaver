// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::json;

use app_domains::flows::models::{Action, Chroot, Condition, Definition, Operator};

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
    assert_eq!(clone.context.as_deref(), Some("work"));
    let Action::Tool(tool) = &clone.action else {
        panic!("expected tool action");
    };
    assert_eq!(tool.tool, "git_clone");
    assert_eq!(tool.scopes, vec!["*"]);
    assert_eq!(tool.arguments["path"], json!("/repo"));

    let when = clone.when.as_ref().unwrap();
    let Condition::All { all } = &when[0] else {
        panic!("expected all");
    };
    assert!(matches!(
        all[1],
        Condition::Compare {
            operator: Operator::Match,
            ..
        }
    ));

    let review = &flow.spec.steps[1];
    assert_eq!(review.context.as_deref(), Some("work"));
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

/// A flow with context `c` and the given steps, indented by four spaces.
fn flow_yaml(steps: &str) -> String {
    format!(
        "version: 1\nmetadata: {{ name: f }}\nspec:\n  contexts:\n    c: {{ agent: x }}\n  steps:\n{}",
        steps
    )
}

const AGENT: &str = "{ name: a, context: c, type: agent, arguments: { prompt: hi } }";

#[test]
fn parses_nested_conditions() {
    let yaml = flow_yaml(
        r#"
    - name: a
      context: c
      type: agent
      when:
        - { left: "{{ Body.region }}", operator: "~=", right: "^eu-" }
        - all:
          - { left: "{{ Body.env }}", operator: "==", right: production }
          - any:
            - { left: "{{ Body.n }}", operator: ">=", right: "3" }
            - { left: "{{ Body.n }}", operator: "!=", right: "0" }
      arguments: { prompt: hi }
"#,
    );
    let flow = parse(&yaml);
    let (ctx, _) = support::temp_context();
    flow.validate(&ctx).unwrap();

    let when = flow.spec.steps[0].when.as_ref().unwrap();
    assert_eq!(when.len(), 2);
    let Condition::All { all } = &when[1] else {
        panic!("expected all");
    };
    let Condition::Any { any } = &all[1] else {
        panic!("expected any");
    };
    assert!(matches!(
        any[0],
        Condition::Compare {
            operator: Operator::Ge,
            ..
        }
    ));
}

#[test]
fn rejects_empty_condition_lists() {
    for when in ["[]", "[{ any: [] }]", "[{ all: [{ any: [] }] }]"] {
        let yaml = flow_yaml(&format!(
            "    - {{ name: a, context: c, type: agent, when: {}, arguments: {{ prompt: hi }} }}",
            when
        ));
        assert!(validation_error(&yaml).contains("is empty"), "{}", when);
    }
}

#[test]
fn rejects_unknown_operators_and_keys() {
    for item in [
        "{ left: a, operator: \"=~\", right: b }",
        "{ left: a, operator: \"==\", right: b, extra: c }",
        "{ none: [{ left: a, operator: \"==\", right: b }] }",
    ] {
        let yaml = flow_yaml(&format!(
            "    - {{ name: a, context: c, type: agent, when: [{}], arguments: {{ prompt: hi }} }}",
            item
        ));
        parse_error(&yaml);
    }
}

#[test]
fn compares_with_operators() {
    let (ctx, _) = support::temp_context();
    assert!(Operator::Eq.compare(&ctx, "a", "a").unwrap());
    assert!(Operator::Ne.compare(&ctx, "a", "b").unwrap());
    assert!(Operator::Lt.compare(&ctx, "2", "10").unwrap());
    assert!(Operator::Le.compare(&ctx, " 3 ", "3.0").unwrap());
    assert!(!Operator::Gt.compare(&ctx, "2", "10").unwrap());
    assert!(Operator::Ge.compare(&ctx, "10", "2").unwrap());
    assert!(
        Operator::Match
            .compare(&ctx, "LGTM, Approved", "(?i)approved")
            .unwrap()
    );
    assert!(
        !Operator::Match
            .compare(&ctx, "rejected", "^approved$")
            .unwrap()
    );

    assert!(
        Operator::Gt
            .compare(&ctx, "abc", "1")
            .unwrap_err()
            .internal_message
            .contains("not a number")
    );
    assert!(
        Operator::Match
            .compare(&ctx, "a", "(")
            .unwrap_err()
            .internal_message
            .contains("invalid regex")
    );
}

const LOOP: &str = r#"
    - name: l
      type: loop
      arguments:
        until: [{ left: "{{ Steps.a.output }}", operator: "~=", right: done }]
        steps:
          - { name: a, context: c, type: agent, arguments: { prompt: hi } }
"#;

#[test]
fn parses_loop_with_default_max() {
    let flow = parse(&flow_yaml(LOOP));
    let (ctx, _) = support::temp_context();
    flow.validate(&ctx).unwrap();

    let Action::Loop(arguments) = &flow.spec.steps[0].action else {
        panic!("expected loop action");
    };
    assert_eq!(arguments.max, 3);
    assert_eq!(arguments.steps.len(), 1);
    assert_eq!(arguments.until.as_ref().unwrap().len(), 1);
}

#[test]
fn rejects_invalid_loops() {
    let with_context = flow_yaml(LOOP).replace("type: loop", "type: loop\n      context: c");
    assert!(validation_error(&with_context).contains("step 'l': a loop has no context"));

    for max in ["0", "101"] {
        let yaml = flow_yaml(LOOP).replace(
            "arguments:\n",
            &format!("arguments:\n        max: {}\n", max),
        );
        assert!(
            validation_error(&yaml).contains("between 1 and 100"),
            "{}",
            max
        );
    }

    let empty = flow_yaml(&format!(
        "    - {{ name: l, type: loop, arguments: {{ steps: [] }} }}\n    - {}",
        AGENT
    ));
    assert!(validation_error(&empty).contains("arguments.steps is empty"));

    let empty_until = flow_yaml(LOOP).replace(
        "until: [{ left: \"{{ Steps.a.output }}\", operator: \"~=\", right: done }]",
        "until: []",
    );
    assert!(validation_error(&empty_until).contains("arguments.until is empty"));
}

#[test]
fn rejects_agent_step_without_context() {
    let yaml = flow_yaml(&format!(
        "    - {{ name: b, type: agent, arguments: {{ prompt: hi }} }}\n    - {}",
        AGENT
    ));
    assert!(validation_error(&yaml).contains("step 'b': context is missing"));
}

#[test]
fn checks_names_and_contexts_inside_loops() {
    let duplicate = flow_yaml(&format!("{}    - {}\n", LOOP, AGENT));
    assert!(validation_error(&duplicate).contains("step name 'a' is used more than once"));

    let unknown = flow_yaml(LOOP).replace("context: c", "context: d");
    assert!(validation_error(&unknown).contains("step 'a' uses unknown context 'd'"));

    // `c` is only used inside the loop, which counts.
    let (ctx, _) = support::temp_context();
    parse(&flow_yaml(LOOP)).validate(&ctx).unwrap();
}
