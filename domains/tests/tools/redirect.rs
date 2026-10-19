// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use app_adapters::{
    agent,
    configuration::ConfigurationMap,
    inference::Scripted,
    logger::LoggerRecording,
    tools::{output_writer, provider::EchoProvider},
};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{
        Inference,
        models::{
            Agent, AgentCacheLevel, AgentInference, Compression, Conversation, Model, Options,
        },
    },
    tools::{
        Tools,
        models::{Call, CallValue, Schema, SchemaField, SchemaFieldType, Tool, ToolPermission},
        traits::ToolsProvider,
    },
};

/// `arguments` returns the arguments it received, `own` does too but defines
/// `redirect_to_file` itself.
struct ArgumentsProvider;

impl ArgumentsProvider {
    fn tool(name: &str) -> Tool {
        let mut schema = Schema::new(name, "");
        schema.add_property(
            "text",
            false,
            SchemaField::new(SchemaFieldType::String, "", None),
        );
        if name == "own" {
            schema.add_property(
                "redirect_to_file",
                false,
                SchemaField::new(SchemaFieldType::String, "Its own argument.", None),
            );
        }
        Tool::new(name.to_string(), String::new(), schema)
    }
}

#[async_trait]
impl ToolsProvider for ArgumentsProvider {
    async fn reload(&mut self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, _ctx: &AppContext) -> Result<Vec<Tool>, AppError> {
        Ok(vec![Self::tool("arguments"), Self::tool("own")])
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Tool, AppError> {
        match name {
            "arguments" | "own" => Ok(Self::tool(name)),
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
        _conversation: &mut Conversation,
        _permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        let mut keys: Vec<&String> = input.arguments.keys().collect();
        keys.sort();
        Ok(format!("keys: {:?}", keys))
    }
}

struct Fixture {
    ctx: AppContext,
    inference: Inference,
    tools: Tools,
    writer: Arc<output_writer::Memory>,
    conversation: Conversation,
    chroot: tempfile::TempDir,
}

async fn fixture(redirect_output: bool) -> Fixture {
    let chroot = tempfile::tempdir().unwrap();
    std::fs::create_dir(chroot.path().join("out")).unwrap();
    let ctx = AppContext::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
        Arc::new(LoggerRecording::new()),
    );

    let mut agent = Agent::new(
        "agent".to_string(),
        "agent".to_string(),
        String::new(),
        String::new(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: AgentCacheLevel::default(),
            options: Options::default(),
            compression: Compression::default(),
        },
    );
    agent.tools.permissions = vec![ToolPermission {
        name: "^.*$".to_string(),
        scopes: vec![],
    }];
    agent.tools.redirect_output = redirect_output;

    let inference = Inference::new(
        Arc::new(agent::Memory::new(vec![agent])),
        vec![Arc::new(Scripted::new())],
    )
    .unwrap();
    let writer = Arc::new(output_writer::Memory::new());
    let tools = Tools::new(
        vec![
            Arc::new(EchoProvider::new(&ctx)),
            Arc::new(ArgumentsProvider),
        ],
        writer.clone(),
    )
    .unwrap();
    let conversation = inference
        .new_conversation(&ctx, "agent", &tools, &chroot.path().to_string_lossy())
        .await
        .unwrap();

    Fixture {
        ctx,
        inference,
        tools,
        writer,
        conversation,
        chroot,
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

    fn host_path(&self, path: &str) -> String {
        self.chroot
            .path()
            .canonicalize()
            .unwrap()
            .join(path.trim_start_matches('/'))
            .to_string_lossy()
            .to_string()
    }
}

fn offers_redirect(conversation: &Conversation, name: &str) -> bool {
    conversation
        .tools()
        .iter()
        .find(|tool| tool.name == name)
        .unwrap()
        .schema
        .properties
        .contains_key("redirect_to_file")
}

#[tokio::test]
async fn schema_offers_redirect_only_when_enabled() {
    let enabled = fixture(true).await;
    assert!(offers_redirect(&enabled.conversation, "echo"));
    assert!(!offers_redirect(&enabled.conversation, "subagent_invoke"));
    let own = enabled
        .conversation
        .tools()
        .iter()
        .find(|tool| tool.name == "own")
        .unwrap();
    assert_eq!(
        own.schema.properties["redirect_to_file"].description,
        "Its own argument."
    );

    let disabled = fixture(false).await;
    assert!(!offers_redirect(&disabled.conversation, "echo"));
}

#[tokio::test]
async fn redirect_fails_when_disabled() {
    let mut fixture = fixture(false).await;
    let message = fixture
        .call(call(
            "echo",
            &[("text", "hi"), ("redirect_to_file", "/out/a.txt")],
        ))
        .await;

    assert!(message.contains("Error"));
    assert!(message.contains("Output redirection is not enabled for this agent"));
    assert!(fixture.writer.writes().is_empty());
}

#[tokio::test]
async fn output_is_written_and_only_a_receipt_is_returned() {
    let mut fixture = fixture(true).await;
    let message = fixture
        .call(call(
            "echo",
            &[("text", "hi"), ("redirect_to_file", "/out/a.txt")],
        ))
        .await;

    let output = format!("echo hi @ {}", fixture.conversation.chroot());
    assert_eq!(
        fixture.writer.writes(),
        vec![(fixture.host_path("/out/a.txt"), output.clone())]
    );
    assert!(message.starts_with("Name: echo\nArguments: "));
    assert!(message.contains(r#""redirect_to_file":"/out/a.txt""#));
    assert!(message.contains(r#""text":"hi""#));
    assert!(message.ends_with(&format!(
        "Result: \nOutput written to '/out/a.txt' ({} bytes, 1 lines)",
        output.len()
    )));
    assert!(!message.contains(&output));
}

#[tokio::test]
async fn nothing_is_written_when_the_tool_fails() {
    let mut fixture = fixture(true).await;
    let message = fixture
        .call(call("fail", &[("redirect_to_file", "/out/a.txt")]))
        .await;

    assert!(message.contains("Error"));
    assert!(message.contains("failed on purpose"));
    assert!(fixture.writer.writes().is_empty());
}

#[tokio::test]
async fn paths_outside_the_chroot_are_rejected() {
    let mut fixture = fixture(true).await;
    for path in [
        "out/a.txt",
        "/../a.txt",
        "/out/../../a.txt",
        "/missing/a.txt",
        "/out",
        "/",
    ] {
        let message = fixture
            .call(call("echo", &[("redirect_to_file", path)]))
            .await;
        assert!(message.contains("Error: "), "{}: {}", path, message);
    }

    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), fixture.chroot.path().join("escape")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("file.txt"),
        fixture.chroot.path().join("out/link.txt"),
    )
    .unwrap();
    std::fs::write(outside.path().join("file.txt"), "").unwrap();
    for path in ["/escape/a.txt", "/out/link.txt"] {
        let message = fixture
            .call(call("echo", &[("redirect_to_file", path)]))
            .await;
        assert!(
            message.contains("outside of chroot"),
            "{}: {}",
            path,
            message
        );
    }

    assert!(fixture.writer.writes().is_empty());
}

#[tokio::test]
async fn the_argument_is_stripped_before_the_provider_sees_it() {
    let mut fixture = fixture(true).await;
    fixture
        .call(call(
            "arguments",
            &[("text", "hi"), ("redirect_to_file", "/out/a.txt")],
        ))
        .await;
    assert_eq!(fixture.writer.writes()[0].1, r#"keys: ["text"]"#);
}

#[tokio::test]
async fn tools_defining_the_argument_are_left_alone() {
    let mut fixture = fixture(true).await;
    let message = fixture
        .call(call("own", &[("redirect_to_file", "/out/a.txt")]))
        .await;

    assert!(
        message.ends_with(r#"keys: ["redirect_to_file"]"#),
        "{}",
        message
    );
    assert!(fixture.writer.writes().is_empty());
}

#[tokio::test]
async fn flow_tool_steps_do_not_redirect() {
    let mut fixture = fixture(true).await;
    let result = fixture
        .tools
        .execute_direct(
            &fixture.ctx,
            &mut fixture.conversation,
            ToolPermission {
                name: "echo".to_string(),
                scopes: vec![],
            },
            &call("echo", &[("redirect_to_file", "/out/a.txt")]),
        )
        .await;

    assert!(result.is_err());
    assert!(fixture.writer.writes().is_empty());
}
