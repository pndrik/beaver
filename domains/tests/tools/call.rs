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
        models::{
            AppContext, AppError,
            file::{self, Chroot},
        },
        traits::Logging,
    },
    inference::{
        Inference,
        models::{Agent, AgentInference, CacheLevel, Compression, Conversation, Model, Options},
        traits::AgentProvider,
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

const RULES: &str = r"
redirect_output: true
allow: [{ name: ^git_.*$ }]
deny: [{ name: git_push }]
";

/// Offers `git_push`, `git_status` and `file_read` and counts executions.
struct CountingProvider {
    executed: Mutex<Vec<String>>,
}

const TOOLS: &[&str] = &["git_push", "git_status", "file_read"];

impl CountingProvider {
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
impl ToolsProvider for CountingProvider {
    async fn reload(&mut self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, _ctx: &AppContext) -> Result<Vec<Tool>, AppError> {
        Ok(TOOLS.iter().map(|name| Self::tool(name)).collect())
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Tool, AppError> {
        if TOOLS.contains(&name) {
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
        _permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        self.executed.lock().unwrap().push(input.name.clone());
        Ok("done".to_string())
    }
}

fn agent(name: &str, prompt: &str, rules: &str) -> Agent {
    let mut agent = Agent::new(
        name.to_string(),
        name.to_string(),
        String::new(),
        prompt.to_string(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: CacheLevel::default(),
            options: Options::default(),
            compression: Compression::default(),
        },
    );
    agent.spec.tools = yaml_serde::from_str::<ToolsConfiguration>(rules).unwrap();
    agent
}

struct Fixture {
    ctx: AppContext,
    logger: Memory,
    provider: Arc<CountingProvider>,
    inference: Inference,
    tools: Tools,
    conversation: Conversation,
}

async fn fixture() -> Fixture {
    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )));
    let logger = Memory::new();
    logger.init(&ctx, "trace").unwrap();
    let provider = Arc::new(CountingProvider {
        executed: Mutex::new(vec![]),
    });
    let inference = Inference::new(
        Arc::new(agent::Memory::new(vec![agent("coder", "You help.", RULES)])),
        vec![Arc::new(Scripted::new())],
    )
    .unwrap();
    let tools = Tools::new(vec![provider.clone()]).unwrap();
    let memory = Arc::new(file_system::Memory::new());
    let chroot = Chroot::new(&ctx, memory, "/", false).await.unwrap();
    chroot.mkdir(&ctx, "/out", false).await.unwrap();
    let conversation = inference
        .new_conversation(&ctx, "coder", &tools, chroot)
        .await
        .unwrap();

    Fixture {
        ctx,
        logger,
        provider,
        inference,
        tools,
        conversation,
    }
}

fn call(name: &str, arguments: &[(&str, &str)]) -> Call {
    Call::new(
        name.to_string(),
        arguments
            .iter()
            .map(|(key, value)| (key.to_string(), CallValue::String(value.to_string())))
            .collect::<HashMap<_, _>>(),
    )
}

impl Fixture {
    async fn call(&mut self, call: Call) -> String {
        self.tools
            .call(&self.ctx, &mut self.conversation, &self.inference, &call)
            .await
            .unwrap();
        self.conversation
            .last()
            .and_then(|message| message.content())
            .unwrap()
            .to_string()
    }

    fn executed(&self) -> Vec<String> {
        self.provider.executed.lock().unwrap().clone()
    }

    fn denied(&self, tool: &str, reason: &str) -> bool {
        self.logger.contains(
            Level::WARN,
            "No permission to access tool",
            &[
                ("action", "call_denied"),
                ("agent", "coder"),
                ("tool", tool),
                ("reason", reason),
            ],
        )
    }
}

#[tokio::test]
async fn a_denied_tool_is_rejected_without_running() {
    let mut fixture = fixture().await;
    let message = fixture.call(call("git_push", &[])).await;

    assert!(message.contains("Error"), "{message}");
    assert!(
        message.contains("permission to call tool: git_push"),
        "{message}"
    );
    assert_eq!(fixture.executed(), [] as [String; 0]);
    assert!(fixture.denied("git_push", "denied"));
}

#[tokio::test]
async fn an_unlisted_tool_is_rejected_without_running() {
    let mut fixture = fixture().await;
    let message = fixture.call(call("file_read", &[])).await;

    assert!(
        message.contains("permission to call tool: file_read"),
        "{message}"
    );
    assert_eq!(fixture.executed(), [] as [String; 0]);
    assert!(fixture.denied("file_read", "not_allowed"));
}

#[tokio::test]
async fn an_allowed_tool_runs() {
    let mut fixture = fixture().await;
    let message = fixture.call(call("git_status", &[])).await;

    assert!(message.ends_with("Result: \ndone"), "{message}");
    assert_eq!(fixture.executed(), ["git_status"]);
}

#[tokio::test]
async fn a_denied_call_writes_no_redirect_file() {
    let mut fixture = fixture().await;
    let message = fixture
        .call(call(
            "git_push",
            &[("harness_redirect_to_file", "/out/a.txt")],
        ))
        .await;

    assert!(
        message.contains("permission to call tool: git_push"),
        "{message}"
    );
    assert_eq!(fixture.executed(), [] as [String; 0]);
    let files = fixture
        .conversation
        .chroot()
        .find(
            &fixture.ctx,
            "/",
            None,
            &[file::Kind::File],
            0..=usize::MAX,
            &[],
            None,
        )
        .await
        .unwrap();
    assert_eq!(files.len(), 0);
}

#[tokio::test]
async fn only_permitted_tools_are_offered_and_redirect_wrapped() {
    let fixture = fixture().await;
    let offered: Vec<&str> = fixture
        .conversation
        .tools()
        .iter()
        .map(|tool| tool.name.as_str())
        .collect();
    assert_eq!(offered, ["git_status"]);
    assert!(
        fixture.conversation.tools()[0]
            .schema
            .properties
            .contains_key("harness_redirect_to_file")
    );
}

/// Returns its agents without validating them.
struct UncheckedProvider(Agent);

#[async_trait]
impl AgentProvider for UncheckedProvider {
    async fn reload(&self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, _ctx: &AppContext) -> Result<Vec<Agent>, AppError> {
        Ok(vec![self.0.clone()])
    }

    async fn get(&self, _ctx: &AppContext, _name: &str) -> Result<Agent, AppError> {
        Ok(self.0.clone())
    }
}

#[tokio::test]
async fn new_conversation_rejects_an_invalid_agent() {
    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )));
    let tools = Tools::new(vec![]).unwrap();
    for (agent, message) in [
        (
            agent("coder", "You help.", "allow: [{ name: 'git_.*' }]"),
            "Agent 'coder' is invalid: spec.tools.allow[0]: name 'git_.*' contains regex syntax; patterns must be written as ^...$",
        ),
        (
            agent("coder", "", "{}"),
            "Agent 'coder' is invalid: spec.prompt is empty",
        ),
    ] {
        let inference = Inference::new(
            Arc::new(UncheckedProvider(agent)),
            vec![Arc::new(Scripted::new())],
        )
        .unwrap();
        let chroot = Chroot::new(&ctx, Arc::new(file_system::Memory::new()), "/", false)
            .await
            .unwrap();
        let Err(err) = inference
            .new_conversation(&ctx, "coder", &tools, chroot)
            .await
        else {
            panic!("expected agent_invalid");
        };
        assert_eq!(err.translation_key, "agent_invalid");
        assert_eq!(err.internal_message, message);
    }
}
