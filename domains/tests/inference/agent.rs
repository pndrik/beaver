// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_domains::inference::models::{Agent, MessageCacheLevel, Model};

use crate::support;

const EXAMPLES: &[(&str, &str)] = &[
    (
        "analyst",
        include_str!("../../../examples/agents/analyst.yaml"),
    ),
    (
        "calculator",
        include_str!("../../../examples/agents/calculator.yaml"),
    ),
    ("coder", include_str!("../../../examples/agents/coder.yaml")),
    (
        "conductor",
        include_str!("../../../examples/agents/conductor.yaml"),
    ),
    (
        "dispatcher",
        include_str!("../../../examples/agents/dispatcher.yaml"),
    ),
    (
        "slacker",
        include_str!("../../../examples/agents/slacker.yaml"),
    ),
    (
        "worker",
        include_str!("../../../examples/agents/worker.yaml"),
    ),
];

const MINIMAL: &str = r"
version: 1
metadata: { name: coder, display_name: Jack, description: Writes code. }
spec:
  inference: { model: claude-opus-4-8 }
  prompt: You help.
";

fn parse(yaml: &str) -> Agent {
    yaml_serde::from_str(yaml).unwrap()
}

fn parse_error(yaml: &str) -> String {
    yaml_serde::from_str::<Agent>(yaml).unwrap_err().to_string()
}

fn validation_error(yaml: &str) -> String {
    let ctx = support::context();
    let err = parse(yaml).validate(&ctx).unwrap_err();
    assert_eq!(err.translation_key, "agent_invalid");
    err.internal_message
}

/// [`MINIMAL`] with `from` replaced by `to`.
fn minimal_with(from: &str, to: &str) -> String {
    assert!(MINIMAL.contains(from), "{from}");
    MINIMAL.replace(from, to)
}

#[test]
fn every_example_parses_and_validates() {
    let ctx = support::context();
    for (name, yaml) in EXAMPLES {
        let agent = parse(yaml);
        assert_eq!(agent.version, 1, "{name}");
        assert_eq!(agent.metadata.name, *name);
        assert!(!agent.spec.prompt.trim().is_empty(), "{name}");
        agent.validate(&ctx).unwrap();
    }
}

#[test]
fn coder_denies_deleting_branches() {
    let coder = parse(EXAMPLES[2].1);
    let tools = &coder.spec.tools;
    assert!(tools.redirect_output);
    assert!(tools.permission("git_branch_delete").is_none());
    assert!(tools.is_denied("git_branch_delete"));
    assert!(tools.permission("git_push").is_some());
    assert_eq!(
        tools.permission("subagent_invoke").unwrap().scopes,
        ["_list", "_invoke", "worker"]
    );
}

#[test]
fn directory_agents_keep_their_prompt() {
    let conductor = parse(EXAMPLES[3].1);
    assert!(
        conductor
            .spec
            .prompt
            .starts_with("You are Gary — the Conductor")
    );
    assert!(
        conductor
            .spec
            .prompt
            .contains("3. The expected output format")
    );
    assert_eq!(
        conductor.spec.inference.caching.system,
        Some(MessageCacheLevel::Ephemeral1h)
    );

    let worker = parse(EXAMPLES[6].1);
    assert!(worker.spec.prompt.starts_with("You are Bert"));
    assert!(worker.spec.prompt.ends_with("not a polished failure.\n"));
}

#[test]
fn a_minimal_agent_gets_the_defaults() {
    let agent = parse(MINIMAL);
    assert_eq!(agent.metadata.display_name, "Jack");
    assert_eq!(agent.metadata.description, "Writes code.");
    assert_eq!(agent.model(), &Model::ClaudeOpus4_8);
    assert!(agent.spec.tools.allow.is_empty());
    assert!(agent.spec.tools.deny.is_empty());
    assert!(!agent.spec.tools.redirect_output);
    assert!(!agent.spec.inference.compression.enabled);
    assert_eq!(agent.spec.inference.options.max_tokens, 4096);
    agent.validate(&support::context()).unwrap();
}

#[test]
fn deny_without_allow_is_accepted() {
    let agent = parse(&minimal_with(
        "  prompt: You help.",
        "  tools: { deny: [{ name: no_such_tool }] }\n  prompt: You help.",
    ));
    agent.validate(&support::context()).unwrap();
    assert!(agent.spec.tools.permission("no_such_tool").is_none());
}

#[test]
fn a_missing_version_is_a_parse_error() {
    let err = parse_error(&minimal_with("version: 1\n", ""));
    assert!(err.contains("missing field `version`"), "{err}");
}

#[test]
fn the_old_flat_layout_is_rejected() {
    for key in [
        "inference: { model: claude-opus-4-8 }",
        "tools: {}",
        "prompt: You help.",
    ] {
        let name = key.split(':').next().unwrap();
        let err = parse_error(&format!("{MINIMAL}{key}\n"));
        assert!(err.contains(&format!("unknown field `{name}`")), "{err}");
    }

    let flat = r"
metadata: { name: coder, display_name: Jack, description: Writes code. }
inference: { model: claude-opus-4-8 }
prompt: You help.
";
    let err = parse_error(flat);
    assert!(err.contains("unknown field `inference`"), "{err}");
}

#[test]
fn old_tool_keys_are_rejected() {
    for key in ["permissions", "toolsets"] {
        let err = parse_error(&minimal_with(
            "  prompt: You help.",
            &format!("  tools: {{ {key}: [] }}\n  prompt: You help."),
        ));
        assert!(err.contains(&format!("unknown field `{key}`")), "{err}");
    }
}

#[test]
fn unknown_keys_are_rejected_at_every_level() {
    let cases = [
        ("version: 1\n", "version: 1\nextra: 1\n"),
        (
            "description: Writes code. }",
            "description: Writes code., extra: 1 }",
        ),
        ("  prompt: You help.", "  prompt: You help.\n  extra: 1"),
        (
            "{ model: claude-opus-4-8 }",
            "{ model: claude-opus-4-8, extra: 1 }",
        ),
        (
            "{ model: claude-opus-4-8 }",
            "{ model: claude-opus-4-8, caching: { default: none, extra: 1 } }",
        ),
        (
            "{ model: claude-opus-4-8 }",
            "{ model: claude-opus-4-8, options: { extra: 1 } }",
        ),
        (
            "{ model: claude-opus-4-8 }",
            "{ model: claude-opus-4-8, compression: { extra: 1 } }",
        ),
        (
            "  prompt: You help.",
            "  tools: { extra: 1 }\n  prompt: You help.",
        ),
        (
            "  prompt: You help.",
            "  tools: { allow: [{ name: a, extra: 1 }] }\n  prompt: You help.",
        ),
        (
            "  prompt: You help.",
            "  tools: { deny: [{ name: a, extra: 1 }] }\n  prompt: You help.",
        ),
        (
            "  prompt: You help.",
            "  tools: { deny: [{ name: a, scopes: [x] }] }\n  prompt: You help.",
        ),
    ];
    for (from, to) in cases {
        let err = parse_error(&minimal_with(from, to));
        assert!(err.contains("unknown field `"), "{to}: {err}");
        let key = if to.contains("scopes: [x]") {
            "scopes"
        } else {
            "extra"
        };
        assert!(err.contains(&format!("`{key}`")), "{to}: {err}");
    }
}

#[test]
fn spec_prompt_and_inference_are_required() {
    let err = parse_error(&minimal_with("  prompt: You help.\n", ""));
    assert!(err.contains("missing field `prompt`"), "{err}");

    let err = parse_error(&minimal_with(
        "  inference: { model: claude-opus-4-8 }\n",
        "",
    ));
    assert!(err.contains("missing field `inference`"), "{err}");
}

#[test]
fn metadata_fields_are_required() {
    for field in ["display_name: Jack, ", "description: Writes code. "] {
        let name = field.split(':').next().unwrap();
        let err = parse_error(&minimal_with(field, ""));
        assert!(err.contains(&format!("missing field `{name}`")), "{err}");
    }
}

#[test]
fn only_version_1_is_supported() {
    assert_eq!(
        validation_error(&minimal_with("version: 1", "version: 2")),
        "Agent 'coder' is invalid: unsupported version 2, expected 1"
    );
}

#[test]
fn an_empty_name_is_rejected() {
    assert_eq!(
        validation_error(&minimal_with("name: coder", "name: ''")),
        "Agent '' is invalid: metadata.name is missing"
    );
}

#[test]
fn a_blank_prompt_is_rejected() {
    for prompt in ["''", "\"  \\n\""] {
        assert_eq!(
            validation_error(&minimal_with(
                "prompt: You help.",
                &format!("prompt: {prompt}")
            )),
            "Agent 'coder' is invalid: spec.prompt is empty",
            "{prompt}"
        );
    }
}

#[test]
fn a_bad_rule_names_its_location() {
    assert_eq!(
        validation_error(&minimal_with(
            "  prompt: You help.",
            "  tools: { allow: [{ name: a }, { name: b }, { name: '^c' }] }\n  prompt: You help.",
        )),
        "Agent 'coder' is invalid: spec.tools.allow[2]: name '^c' must end with '$'"
    );
    assert_eq!(
        validation_error(&minimal_with(
            "  prompt: You help.",
            "  tools: { deny: [{ name: a }, { name: '^git_' }] }\n  prompt: You help.",
        )),
        "Agent 'coder' is invalid: spec.tools.deny[1]: name '^git_' must end with '$'"
    );
    assert_eq!(
        validation_error(&minimal_with(
            "  prompt: You help.",
            "  tools: { allow: [{ name: a, scopes: [/x, /y, ''] }] }\n  prompt: You help.",
        )),
        "Agent 'coder' is invalid: spec.tools.allow[0]: scope 2 is empty"
    );
}
