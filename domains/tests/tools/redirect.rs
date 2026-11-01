// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_adapters::context::ContextStandard;
use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use app_adapters::{
    agent, configuration::ConfigurationMap, file_system, inference::Scripted,
    tools::provider::EchoProvider,
};
use app_domains::{
    app_error,
    core::models::{
        AppContext, AppError,
        file::{self, Chroot},
    },
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
/// `harness_redirect_to_file` itself.
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
                "harness_redirect_to_file",
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
        Ok(format!("keys: {keys:?}"))
    }
}

struct Fixture {
    ctx: AppContext,
    inference: Inference,
    tools: Tools,
    conversation: Conversation,
    /// The whole file system, the agent works in `/agent`.
    root: Chroot,
}

/// The agent's chroot is `/agent` with the directory `out`, next to
/// `/secret/file.txt`.
async fn fixture(redirect_output: bool) -> Fixture {
    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )));
    let memory = Arc::new(file_system::Memory::new());
    let root = Chroot::new(&ctx, memory.clone(), "/", false).await.unwrap();
    root.mkdir(&ctx, "/agent/out", true).await.unwrap();
    root.mkdir(&ctx, "/secret", false).await.unwrap();
    root.write(
        &ctx,
        "/secret/file.txt",
        b"secret",
        file::WriteMode::Truncate,
    )
    .await
    .unwrap();
    let chroot = Chroot::new(&ctx, memory, "/agent", false).await.unwrap();

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
    let tools = Tools::new(vec![
        Arc::new(EchoProvider::new(&ctx)),
        Arc::new(ArgumentsProvider),
    ])
    .unwrap();
    let conversation = inference
        .new_conversation(&ctx, "agent", &tools, chroot)
        .await
        .unwrap();

    Fixture {
        ctx,
        inference,
        tools,
        conversation,
        root,
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

    /// The files in the agent's chroot with their content.
    async fn files(&self) -> Vec<(String, String)> {
        let chroot = self.conversation.chroot();
        let files = chroot
            .find(
                &self.ctx,
                "/",
                None,
                &[file::Kind::File],
                0..=usize::MAX,
                &[],
                None,
            )
            .await
            .unwrap();
        let mut contents = vec![];
        for entry in files {
            let content = chroot.read_to_string(&self.ctx, entry.path.as_str()).await;
            contents.push((entry.path.to_string(), content.unwrap()));
        }
        contents
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
        .contains_key("harness_redirect_to_file")
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
    let echo = enabled
        .conversation
        .tools()
        .iter()
        .find(|tool| tool.name == "echo")
        .unwrap();
    assert_eq!(
        own.schema.properties["harness_redirect_to_file"].description,
        echo.schema.properties["harness_redirect_to_file"].description
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
            &[("text", "hi"), ("harness_redirect_to_file", "/out/a.txt")],
        ))
        .await;

    assert!(message.contains("Error"));
    assert!(message.contains("Output redirection is not enabled for this agent"));
    assert_eq!(
        fixture.files().await,
        [] as [(std::string::String, std::string::String); 0]
    );
}

#[tokio::test]
async fn output_is_written_and_only_a_receipt_is_returned() {
    let mut fixture = fixture(true).await;
    let message = fixture
        .call(call(
            "echo",
            &[("text", "hi"), ("harness_redirect_to_file", "/out/a.txt")],
        ))
        .await;

    let output = "echo hi @ /agent".to_string();
    assert_eq!(
        fixture.files().await,
        vec![("/out/a.txt".to_string(), output.clone())]
    );
    assert!(message.starts_with("Name: echo\nArguments: "));
    assert!(message.contains(r#""harness_redirect_to_file":"/out/a.txt""#));
    assert!(message.contains(r#""text":"hi""#));
    assert!(message.ends_with(&format!(
        "}}\nOutput written to '/out/a.txt' ({} bytes, 1 lines)",
        output.len()
    )));
    assert!(!message.contains("Result:"));
    assert!(!message.contains(&output));
}

#[tokio::test]
async fn nothing_is_written_when_the_tool_fails() {
    let mut fixture = fixture(true).await;
    let message = fixture
        .call(call("fail", &[("harness_redirect_to_file", "/out/a.txt")]))
        .await;

    assert!(message.contains("Error"));
    assert!(message.contains("failed on purpose"));
    assert_eq!(
        fixture.files().await,
        [] as [(std::string::String, std::string::String); 0]
    );
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
            .call(call("echo", &[("harness_redirect_to_file", path)]))
            .await;
        assert!(message.contains("Error: "), "{path}: {message}");
    }

    let ctx = &fixture.ctx;
    fixture
        .root
        .symlink(ctx, "/secret", "/agent/escape")
        .await
        .unwrap();
    fixture
        .root
        .symlink(ctx, "/secret/file.txt", "/agent/out/link.txt")
        .await
        .unwrap();
    for path in ["/escape/a.txt", "/out/link.txt"] {
        let message = fixture
            .call(call("echo", &[("harness_redirect_to_file", path)]))
            .await;
        assert!(message.contains("is not accessible"), "{path}: {message}");
    }

    assert_eq!(
        fixture.files().await,
        [] as [(std::string::String, std::string::String); 0]
    );
    let secret = fixture
        .root
        .read_to_string(&fixture.ctx, "/secret/file.txt");
    assert_eq!(secret.await.unwrap(), "secret");
    let entries = fixture
        .root
        .read_dir(&fixture.ctx, "/secret")
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
}

#[tokio::test]
async fn the_argument_is_stripped_before_the_provider_sees_it() {
    let mut fixture = fixture(true).await;
    fixture
        .call(call(
            "arguments",
            &[("text", "hi"), ("harness_redirect_to_file", "/out/a.txt")],
        ))
        .await;
    assert_eq!(fixture.files().await[0].1, r#"keys: ["text"]"#);
}

#[tokio::test]
async fn tools_defining_the_argument_are_overridden() {
    let mut fixture = fixture(true).await;
    let message = fixture
        .call(call(
            "own",
            &[("text", "hi"), ("harness_redirect_to_file", "/out/a.txt")],
        ))
        .await;

    assert!(
        message.contains("Output written to '/out/a.txt'"),
        "{}",
        message
    );
    assert_eq!(
        fixture.files().await,
        vec![("/out/a.txt".to_string(), r#"keys: ["text"]"#.to_string())]
    );
}

#[tokio::test]
async fn flow_tool_steps_do_not_redirect() {
    let mut fixture = fixture(true).await;
    let result = fixture
        .tools
        .call_direct(
            &fixture.ctx,
            &mut fixture.conversation,
            ToolPermission {
                name: "echo".to_string(),
                scopes: vec![],
            },
            &call(
                "echo",
                &[("text", "hi"), ("harness_redirect_to_file", "/out/a.txt")],
            ),
        )
        .await;

    assert_eq!(result.unwrap(), "echo hi @ /agent");
    assert_eq!(
        fixture.files().await,
        [] as [(std::string::String, std::string::String); 0]
    );
}
