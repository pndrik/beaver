// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_adapters::context::ContextStandard;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use tracing::Level;

use app_adapters::{
    agent, configuration::ConfigurationMap, file_system, inference::Scripted, logging::Memory,
};
use app_domains::{
    app_error,
    core::{
        models::{AppContext, AppError, file::Chroot},
        traits::Logging,
    },
    inference::{
        Inference,
        models::{
            Agent, AgentInference, CacheLevel, Compression, Conversation, Message, Model, Options,
        },
    },
    tools::{
        Tools,
        models::{
            Call, CallValue, Schema, SchemaField, SchemaFieldType, Tool, ToolPermission,
            ToolsConfiguration,
        },
        traits::ToolsProvider,
    },
};

fn agent(name: &str, permissions: Vec<ToolPermission>) -> Agent {
    let mut agent = Agent::new(
        name.to_string(),
        name.to_string(),
        String::new(),
        "You help.".to_string(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: CacheLevel::default(),
            options: Options::default(),
            compression: Compression::default(),
        },
    );
    agent.spec.tools.allow = permissions;
    agent
}

struct Fixture {
    ctx: AppContext,
    scripted: Arc<Scripted>,
    inference: Inference,
    tools: Tools,
    conversation: Conversation,
}

/// A conversation with `parent`, which may invoke the subagent `helper`.
/// It works in the chroot `/work`.
async fn fixture() -> Fixture {
    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )));
    let scripted = Arc::new(Scripted::new());
    let parent = agent(
        "parent",
        vec![ToolPermission {
            name: "subagent_invoke".to_string(),
            scopes: vec!["_invoke".to_string(), "helper".to_string()],
        }],
    );
    let inference = Inference::new(
        Arc::new(agent::Memory::new(vec![parent, agent("helper", vec![])])),
        vec![scripted.clone()],
    )
    .unwrap();
    let tools = Tools::new(vec![]).unwrap();
    let memory = Arc::new(file_system::Memory::new());
    let chroot = Chroot::new(&ctx, memory, "/work", true).await.unwrap();
    let conversation = inference
        .new_conversation(&ctx, "parent", &tools, chroot)
        .await
        .unwrap();

    Fixture {
        ctx,
        scripted,
        inference,
        tools,
        conversation,
    }
}

fn invoke(prompt: &str) -> Call {
    Call::new(
        "subagent_invoke".to_string(),
        HashMap::from([
            ("name".to_string(), CallValue::String("helper".to_string())),
            ("prompt".to_string(), CallValue::String(prompt.to_string())),
        ]),
    )
}

#[tokio::test]
async fn subagent_runs_in_the_chroot_of_its_parent() {
    let Fixture {
        ctx,
        scripted,
        inference,
        tools,
        mut conversation,
    } = fixture().await;
    tools
        .call(&ctx, &mut conversation, &inference, &invoke("hi"))
        .await
        .unwrap();

    let calls = scripted.calls();
    let helper = calls.iter().find(|c| c.agent == "helper").unwrap();
    assert_eq!(helper.prompt, "hi");
    assert_eq!(conversation.chroot().root().as_str(), "/work");
    assert_eq!(helper.chroot, "/work");
    assert!(calls.iter().all(|c| c.chroot == "/work"));
}

#[tokio::test]
async fn subagent_messages_carry_usage() {
    let Fixture {
        ctx,
        inference,
        tools,
        mut conversation,
        ..
    } = fixture().await;
    conversation.insert_message(
        Message::user("start".to_string(), conversation.agent()),
        None,
    );
    inference
        .infer(&ctx, &mut conversation, &tools)
        .await
        .unwrap();
    let parent_usage = conversation.usage().unwrap();

    tools
        .call(&ctx, &mut conversation, &inference, &invoke("hi"))
        .await
        .unwrap();

    let agent_messages: Vec<_> = conversation
        .messages()
        .iter()
        .filter_map(|message| message.as_agent())
        .collect();
    let prompt = agent_messages
        .iter()
        .find(|message| message.content.as_deref() == Some("hi"))
        .unwrap();
    assert_eq!(prompt.name, "parent");
    assert_eq!(prompt.usage, parent_usage);

    let reply = agent_messages
        .iter()
        .find(|message| message.name == "helper")
        .unwrap();
    assert!(reply.usage.input > 0);
    assert!(reply.usage.output > 0);
}

/// Offers the tools `a` and `b` and records which ones ran.
struct RecordingProvider {
    executed: Mutex<Vec<String>>,
}

impl RecordingProvider {
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
        Ok(vec![Self::tool("a"), Self::tool("b")])
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Tool, AppError> {
        match name {
            "a" | "b" => Ok(Self::tool(name)),
            _ => Err(app_error!(
                NotFound,
                "tool_not_found",
                "Tool not found.",
                ctx.clone()
            )),
        }
    }

    async fn execute(
        &self,
        _ctx: &AppContext,
        conversation: &mut Conversation,
        _permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        self.executed.lock().unwrap().push(format!(
            "{}@{}",
            input.name,
            conversation.agent().metadata.name
        ));
        Ok("done".to_string())
    }
}

fn agent_with_rules(name: &str, rules: &str) -> Agent {
    let mut agent = agent(name, vec![]);
    agent.spec.tools = yaml_serde::from_str::<ToolsConfiguration>(rules).unwrap();
    agent
}

struct RulesFixture {
    ctx: AppContext,
    logger: Memory,
    scripted: Arc<Scripted>,
    provider: Arc<RecordingProvider>,
    inference: Inference,
    tools: Tools,
    conversation: Conversation,
}

/// A conversation with `parent` and the agents `helper` and `other`, each
/// with its own rules. `other` allows nothing.
async fn rules_fixture(parent: &str, helper: &str) -> RulesFixture {
    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )));
    let logger = Memory::new();
    logger.init(&ctx, "trace").unwrap();
    let scripted = Arc::new(Scripted::new());
    let provider = Arc::new(RecordingProvider {
        executed: Mutex::new(vec![]),
    });
    let inference = Inference::new(
        Arc::new(agent::Memory::new(vec![
            agent_with_rules("parent", parent),
            agent_with_rules("helper", helper),
            agent_with_rules("other", "{}"),
        ])),
        vec![scripted.clone()],
    )
    .unwrap();
    let tools = Tools::new(vec![provider.clone()]).unwrap();
    let memory = Arc::new(file_system::Memory::new());
    let chroot = Chroot::new(&ctx, memory, "/", false).await.unwrap();
    let conversation = inference
        .new_conversation(&ctx, "parent", &tools, chroot)
        .await
        .unwrap();

    RulesFixture {
        ctx,
        logger,
        scripted,
        provider,
        inference,
        tools,
        conversation,
    }
}

impl RulesFixture {
    /// Calls `call` as the parent and returns the last message.
    async fn call(&mut self, call: Call) -> String {
        self.tools
            .call(&self.ctx, &mut self.conversation, &self.inference, &call)
            .await
            .unwrap();
        self.conversation
            .last()
            .and_then(|message| message.content())
            .unwrap_or_default()
            .to_string()
    }

    fn offers(&self, tool: &str) -> bool {
        self.conversation.tools().iter().any(|t| t.name == tool)
    }

    fn executed(&self) -> Vec<String> {
        self.provider.executed.lock().unwrap().clone()
    }

    fn helper_was_called(&self) -> bool {
        self.scripted
            .calls()
            .iter()
            .any(|call| call.agent == "helper")
    }
}

const PARENT_DENIES_INVOKE: &str = r"
allow: [{ name: ^subagent_.*$, scopes: [_list, _invoke, helper] }]
deny: [{ name: subagent_invoke }]
";

fn list() -> Call {
    Call::new("subagent_list".to_string(), HashMap::new())
}

#[tokio::test]
async fn a_denied_subagent_invoke_is_neither_offered_nor_run() {
    let mut fixture = rules_fixture(PARENT_DENIES_INVOKE, "{}").await;
    assert!(!fixture.offers("subagent_invoke"));
    assert!(fixture.offers("subagent_list"));

    let message = fixture.call(invoke("hi")).await;
    assert!(
        message.contains("permission to call tool: subagent_invoke"),
        "{message}"
    );
    assert!(!fixture.helper_was_called());
    assert!(fixture.logger.contains(
        Level::WARN,
        "No permission to access tool",
        &[("tool", "subagent_invoke"), ("reason", "denied")],
    ));

    let message = fixture.call(list()).await;
    assert!(message.starts_with("Available subagents:"), "{message}");
    assert!(message.contains("Name: helper"), "{message}");
    assert!(!message.contains("Name: other"), "{message}");
}

#[tokio::test]
async fn a_denied_subagent_list_is_neither_offered_nor_run() {
    let mut fixture = rules_fixture(
        r"
allow: [{ name: ^subagent_.*$, scopes: [_list, _invoke, helper] }]
deny: [{ name: subagent_list }]
",
        "{}",
    )
    .await;
    assert!(!fixture.offers("subagent_list"));
    assert!(fixture.offers("subagent_invoke"));

    let message = fixture.call(list()).await;
    assert!(
        message.contains("permission to call tool: subagent_list"),
        "{message}"
    );
    assert!(!message.contains("Available subagents"), "{message}");
}

#[tokio::test]
async fn a_subagent_is_checked_against_its_own_rules() {
    let mut fixture = rules_fixture(
        r"
allow:
  - { name: subagent_invoke, scopes: [_invoke, helper] }
  - { name: a }
  - { name: b }
",
        "allow: [{ name: a }]",
    )
    .await;
    assert!(fixture.offers("a") && fixture.offers("b"));

    fixture.call(invoke("call:b x")).await;
    assert_eq!(fixture.executed(), [] as [String; 0]);
    assert!(fixture.logger.contains(
        Level::WARN,
        "No permission to access tool",
        &[
            ("agent", "helper"),
            ("tool", "b"),
            ("reason", "not_allowed")
        ],
    ));

    fixture.call(invoke("call:a x")).await;
    assert_eq!(fixture.executed(), ["a@helper"]);
}

#[tokio::test]
async fn a_subagent_does_not_inherit_a_parent_deny() {
    let mut fixture = rules_fixture(
        r"
allow: [{ name: subagent_invoke, scopes: [_invoke, helper] }, { name: a }]
deny: [{ name: b }]
",
        "allow: [{ name: b }]",
    )
    .await;
    assert!(!fixture.offers("b"));

    fixture.call(invoke("call:b x")).await;
    assert_eq!(fixture.executed(), ["b@helper"]);
}

#[tokio::test]
async fn subagent_scopes_are_checked() {
    for (rules, call, error) in [
        (
            "allow: [{ name: subagent_list, scopes: [helper] }]",
            list(),
            "Can not list subagents without scope '_list'.",
        ),
        (
            "allow: [{ name: subagent_invoke, scopes: [helper] }]",
            invoke("hi"),
            "Can not invoke subagents without scope '_invoke'.",
        ),
        (
            "allow: [{ name: subagent_invoke, scopes: [_invoke, other] }]",
            invoke("hi"),
            "Can not invoke subagent 'helper' without scope 'helper'.",
        ),
    ] {
        let mut fixture = rules_fixture(rules, "{}").await;
        let message = fixture.call(call).await;
        assert!(message.contains(error), "{rules}: {message}");
        assert!(!fixture.helper_was_called(), "{rules}");
    }
}
