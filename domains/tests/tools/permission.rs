// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_adapters::context::ContextStandard;
use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use app_adapters::{agent, configuration::ConfigurationMap, file_system, inference::Scripted};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError, file::Chroot},
    inference::{
        Inference,
        models::{Agent, AgentInference, CacheLevel, Compression, Conversation, Model, Options},
    },
    tools::{
        Tools,
        models::{
            Call, Schema, SchemaField, SchemaFieldType, Tool, ToolDenial, ToolPermission,
            ToolsConfiguration,
        },
        traits::ToolsProvider,
    },
};

/// `(rules, tool, scopes the tool is called with or None if not permitted)`.
/// Every rule here is valid, so the table also runs through `Tools`.
const CASES: &[(&str, &str, Option<&[&str]>)] = &[
    // Literals and near-misses.
    (
        "allow: [{ name: git_push, scopes: [/a] }]",
        "git_push",
        Some(&["/a"]),
    ),
    ("allow: [{ name: git_push }]", "git", None),
    ("allow: [{ name: git }]", "git_push", None),
    ("allow: [{ name: push }]", "git_push", None),
    ("allow: [{ name: git_push }]", "Git_push", None),
    ("allow: [{ name: git_push }]", "git_push_all", None),
    ("allow: [{ name: mcp_a.b-c }]", "mcp_a.b-c", Some(&[])),
    ("allow: [{ name: mcp_a.b-c }]", "mcp_aXb-c", None),
    // Patterns match the whole name.
    ("allow: [{ name: ^git_.*$ }]", "git_push", Some(&[])),
    ("allow: [{ name: ^git_.*$ }]", "xgit_push", None),
    ("allow: [{ name: ^git_.*$ }]", "agit_", None),
    ("allow: [{ name: ^git_.*$ }]", "Git_push", None),
    ("allow: [{ name: ^git_.*|evil$ }]", "notevil", None),
    ("allow: [{ name: ^git_.*|evil$ }]", "evil", Some(&[])),
    ("allow: [{ name: ^git_.*|evil$ }]", "git_push", Some(&[])),
    ("allow: [{ name: ^git$ }]", "git_push", None),
    ("allow: [{ name: ^.*$ }]", "Git_push", Some(&[])),
    ("allow: [{ name: ^.*$ }]", "mcp_a.b-c", Some(&[])),
    // The first matching allow rule wins.
    (
        "allow: [{ name: ^git_.*$, scopes: [/a] }, { name: git_push, scopes: [/b] }]",
        "git_push",
        Some(&["/a"]),
    ),
    (
        "allow: [{ name: git_push, scopes: [/b] }, { name: ^git_.*$, scopes: [/a] }]",
        "git_push",
        Some(&["/b"]),
    ),
    (
        "allow: [{ name: git_push, scopes: [/b] }, { name: ^git_.*$, scopes: [/a] }]",
        "git_status",
        Some(&["/a"]),
    ),
    // Deny wins over allow, wherever it is listed.
    (
        "allow: [{ name: ^git_.*$ }]\ndeny: [{ name: git_push }]",
        "git_push",
        None,
    ),
    (
        "allow: [{ name: ^git_.*$ }]\ndeny: [{ name: git_push }]",
        "git_status",
        Some(&[]),
    ),
    (
        "deny: [{ name: git_push }]\nallow: [{ name: ^git_.*$ }]",
        "git_push",
        None,
    ),
    (
        "deny: [{ name: git_push }]\nallow: [{ name: ^git_.*$ }]",
        "git_status",
        Some(&[]),
    ),
    (
        "allow: [{ name: git_push }]\ndeny: [{ name: ^git_.*$ }]",
        "git_push",
        None,
    ),
    (
        "allow: [{ name: ^.*$ }]\ndeny: [{ name: ^.*$ }]",
        "git_push",
        None,
    ),
    (
        "allow: [{ name: ^.*$ }]\ndeny: [{ name: ^.*$ }]",
        "evil",
        None,
    ),
    (
        "allow: [{ name: git_push, scopes: [/a] }]\ndeny: [{ name: git_status }]",
        "git_push",
        Some(&["/a"]),
    ),
    (
        "allow: [{ name: git_push }]\ndeny: [{ name: Git_push }]",
        "git_push",
        Some(&[]),
    ),
    ("deny: [{ name: git_push }]", "git_push", None),
    ("deny: [{ name: git_push }]", "git_status", None),
    // No rules, nothing permitted.
    ("{}", "git_push", None),
    ("allow: []\ndeny: []", "evil", None),
];

fn configuration(rules: &str) -> ToolsConfiguration {
    yaml_serde::from_str(rules).unwrap()
}

fn context() -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )))
}

#[test]
fn permission_follows_the_table() {
    for (rules, tool, expected) in CASES {
        let configuration = configuration(rules);
        let scopes = configuration
            .permission(tool)
            .map(|permission| permission.scopes.clone());
        let expected = expected.map(|scopes| scopes.iter().map(ToString::to_string).collect());
        assert_eq!(scopes, expected, "{rules} / {tool}");
    }
}

#[test]
fn a_literal_that_looks_like_a_pattern_matches_only_itself() {
    let configuration = configuration("allow: [{ name: 'git_.*' }, { name: 'git_.*$' }]");
    assert!(configuration.permission("git_push").is_none());
    assert!(configuration.permission("git_.*").is_some());
}

#[test]
fn an_invalid_pattern_matches_nothing() {
    let configuration = configuration("allow: [{ name: '^git_($' }]");
    assert!(configuration.permission("git_(").is_none());
    assert!(configuration.permission("git_push").is_none());
}

#[test]
fn is_denied_only_for_denied_tools() {
    let configuration = configuration("allow: [{ name: ^git_.*$ }]\ndeny: [{ name: git_push }]");
    assert!(configuration.is_denied("git_push"));
    assert!(!configuration.is_denied("git_status"));
    assert!(!configuration.is_denied("file_read"));
}

#[test]
fn a_deny_rule_has_no_scopes() {
    let err = yaml_serde::from_str::<ToolsConfiguration>("deny: [{ name: a, scopes: [/x] }]")
        .unwrap_err()
        .to_string();
    assert!(err.contains("unknown field `scopes`"), "{err}");
}

#[test]
fn an_allow_rule_has_no_other_keys() {
    let err = yaml_serde::from_str::<ToolsConfiguration>("allow: [{ name: a, scope: [/x] }]")
        .unwrap_err()
        .to_string();
    assert!(err.contains("unknown field `scope`"), "{err}");
}

#[test]
fn unknown_keys_are_rejected() {
    for key in ["permissions", "toolsets"] {
        let err = yaml_serde::from_str::<ToolsConfiguration>(&format!("{key}: []"))
            .unwrap_err()
            .to_string();
        assert!(err.contains(&format!("unknown field `{key}`")), "{err}");
    }
}

fn allow(name: &str, scopes: &[&str]) -> ToolPermission {
    ToolPermission {
        name: name.to_string(),
        scopes: scopes.iter().map(ToString::to_string).collect(),
    }
}

fn deny(name: &str) -> ToolDenial {
    ToolDenial {
        name: name.to_string(),
    }
}

fn validation_error(configuration: &ToolsConfiguration) -> String {
    let err = configuration
        .validate(&context(), "spec.tools")
        .unwrap_err();
    assert_eq!(err.translation_key, "agent_invalid");
    err.internal_message
}

const INVALID_NAMES: &[(&str, &str)] = &[
    ("", "name is empty"),
    ("^git_", "name '^git_' must end with '$'"),
    ("^git_(", "name '^git_(' must end with '$'"),
    (
        "git_.*",
        "name 'git_.*' contains regex syntax; patterns must be written as ^...$",
    ),
    (
        "git_*",
        "name 'git_*' contains regex syntax; patterns must be written as ^...$",
    ),
    (
        "a|b",
        "name 'a|b' contains regex syntax; patterns must be written as ^...$",
    ),
    (
        "git\\_x",
        "name 'git\\_x' contains regex syntax; patterns must be written as ^...$",
    ),
    (
        "git_.*$",
        "name 'git_.*$' contains regex syntax; patterns must be written as ^...$",
    ),
    (
        "a^b",
        "name 'a^b' contains regex syntax; patterns must be written as ^...$",
    ),
];

#[test]
fn invalid_allow_names_are_rejected() {
    for (name, message) in INVALID_NAMES {
        let configuration = ToolsConfiguration {
            allow: vec![allow(name, &[])],
            ..ToolsConfiguration::default()
        };
        assert_eq!(
            validation_error(&configuration),
            format!("spec.tools.allow[0]: {message}")
        );
    }
}

#[test]
fn invalid_deny_names_are_rejected() {
    for (name, message) in INVALID_NAMES {
        let configuration = ToolsConfiguration {
            deny: vec![deny(name)],
            ..ToolsConfiguration::default()
        };
        assert_eq!(
            validation_error(&configuration),
            format!("spec.tools.deny[0]: {message}")
        );
    }
}

#[test]
fn an_invalid_regex_is_rejected() {
    for configuration in [
        ToolsConfiguration {
            allow: vec![allow("^git_($", &[])],
            ..ToolsConfiguration::default()
        },
        ToolsConfiguration {
            deny: vec![deny("^git_($")],
            ..ToolsConfiguration::default()
        },
    ] {
        let message = validation_error(&configuration);
        assert!(
            message.starts_with("spec.tools.allow[0]: name '^git_($' is not a valid regex: ")
                || message.starts_with("spec.tools.deny[0]: name '^git_($' is not a valid regex: "),
            "{message}"
        );
    }
}

#[test]
fn an_empty_scope_is_rejected() {
    let configuration = ToolsConfiguration {
        allow: vec![allow("a", &["/x", "/y", ""])],
        ..ToolsConfiguration::default()
    };
    assert_eq!(
        validation_error(&configuration),
        "spec.tools.allow[0]: scope 2 is empty"
    );
}

#[test]
fn the_location_names_the_index_of_the_rule() {
    let configuration = ToolsConfiguration {
        allow: vec![allow("a", &[]), allow("^b.*$", &[]), allow("^c", &[])],
        deny: vec![deny("d"), deny("e*")],
        ..ToolsConfiguration::default()
    };
    assert_eq!(
        validation_error(&configuration),
        "spec.tools.allow[2]: name '^c' must end with '$'"
    );

    let configuration = ToolsConfiguration {
        allow: vec![allow("a", &[])],
        deny: vec![deny("d"), deny("e*")],
        ..ToolsConfiguration::default()
    };
    assert_eq!(
        validation_error(&configuration),
        "spec.tools.deny[1]: name 'e*' contains regex syntax; patterns must be written as ^...$"
    );
}

#[test]
fn valid_rules_are_accepted() {
    let ctx = context();
    for rules in [
        "allow: [{ name: mcp_a.b-c }, { name: ^git_.*$, scopes: [/repo] }]",
        "deny: [{ name: git_push }]",
        "allow: [{ name: ^.*$ }]\ndeny: [{ name: no_such_tool }]",
        "{}",
    ] {
        configuration(rules).validate(&ctx, "spec.tools").unwrap();
    }
    for (rules, _, _) in CASES {
        configuration(rules).validate(&ctx, "spec.tools").unwrap();
    }
}

/// Offers one tool per name in [`CASES`] and records `(tool, scopes)` for
/// every execution.
struct RecordingProvider {
    executed: Mutex<Vec<(String, Vec<String>)>>,
}

impl RecordingProvider {
    fn names() -> BTreeSet<&'static str> {
        CASES.iter().map(|(_, tool, _)| *tool).collect()
    }

    fn tool(name: &str) -> Tool {
        let mut schema = Schema::new(name, "");
        schema.add_property(
            "text",
            false,
            SchemaField::new(SchemaFieldType::String, "", None),
        );
        Tool::new(name.to_string(), String::new(), schema)
    }
}

#[async_trait]
impl ToolsProvider for RecordingProvider {
    async fn reload(&mut self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, _ctx: &AppContext) -> Result<Vec<Tool>, AppError> {
        Ok(Self::names().into_iter().map(Self::tool).collect())
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Tool, AppError> {
        if Self::names().contains(name) {
            return Ok(Self::tool(name));
        }
        Err(app_error!(
            NotFound,
            "tool_not_found",
            "Tool not found.",
            ctx.clone()
        ))
    }

    async fn execute(
        &self,
        _ctx: &AppContext,
        _conversation: &mut Conversation,
        permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        self.executed
            .lock()
            .unwrap()
            .push((input.name.clone(), permissions.scopes));
        Ok("done".to_string())
    }
}

fn agent(tools: ToolsConfiguration) -> Agent {
    let mut agent = Agent::new(
        "agent".to_string(),
        "agent".to_string(),
        String::new(),
        "You help.".to_string(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: CacheLevel::default(),
            options: Options::default(),
            compression: Compression::default(),
        },
    );
    agent.spec.tools = tools;
    agent
}

#[tokio::test]
async fn offered_and_callable_tools_follow_the_table() {
    let ctx = context();
    for (rules, tool, expected) in CASES {
        let provider = Arc::new(RecordingProvider {
            executed: Mutex::new(vec![]),
        });
        let inference = Inference::new(
            Arc::new(agent::Memory::new(vec![agent(configuration(rules))])),
            vec![Arc::new(Scripted::new())],
        )
        .unwrap();
        let tools = Tools::new(vec![provider.clone()]).unwrap();
        let chroot = Chroot::new(&ctx, Arc::new(file_system::Memory::new()), "/", false)
            .await
            .unwrap();
        let mut conversation = inference
            .new_conversation(&ctx, "agent", &tools, chroot)
            .await
            .unwrap();

        let offered = conversation.tools().iter().any(|t| t.name == *tool);
        assert_eq!(offered, expected.is_some(), "offered: {rules} / {tool}");

        let call = Call::new(tool.to_string(), HashMap::new());
        tools
            .call(&ctx, &mut conversation, &inference, &call)
            .await
            .unwrap();
        let executed = provider.executed.lock().unwrap().clone();
        let expected: Vec<(String, Vec<String>)> = expected
            .map(|scopes| {
                vec![(
                    tool.to_string(),
                    scopes.iter().map(ToString::to_string).collect(),
                )]
            })
            .unwrap_or_default();
        assert_eq!(executed, expected, "called: {rules} / {tool}");
    }
}
