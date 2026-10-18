// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{collections::HashMap, sync::Arc};

use app_adapters::{
    agent, configuration::ConfigurationMap, inference::Scripted, logger::LoggerRecording,
};
use app_domains::{
    core::models::AppContext,
    inference::{
        Inference,
        models::{
            Agent, AgentCacheLevel, AgentInference, Compression, Conversation, Message, Model,
            Options,
        },
    },
    tools::{
        Tools,
        models::{Call, CallValue, ToolPermission},
    },
};

fn agent(name: &str, permissions: Vec<ToolPermission>) -> Agent {
    let mut agent = Agent::new(
        name.to_string(),
        name.to_string(),
        String::new(),
        String::new(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: AgentCacheLevel::default(),
            options: Options::default(),
            compression: Compression::default(),
        },
    );
    agent.tools.permissions = permissions;
    agent
}

struct Fixture {
    ctx: AppContext,
    scripted: Arc<Scripted>,
    inference: Inference,
    tools: Tools,
    conversation: Conversation,
    _chroot: tempfile::TempDir,
}

/// A conversation with `parent`, which may invoke the subagent `helper`.
async fn fixture() -> Fixture {
    let chroot = tempfile::tempdir().unwrap();
    let ctx = AppContext::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
        Arc::new(LoggerRecording::new()),
    );
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
    let conversation = inference
        .new_conversation(&ctx, "parent", &tools, &chroot.path().to_string_lossy())
        .await
        .unwrap();

    Fixture {
        ctx,
        scripted,
        inference,
        tools,
        conversation,
        _chroot: chroot,
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
        _chroot,
    } = fixture().await;
    tools
        .call(&ctx, &mut conversation, &inference, &invoke("hi"))
        .await
        .unwrap();

    let calls = scripted.calls();
    let helper = calls.iter().find(|c| c.agent == "helper").unwrap();
    assert_eq!(helper.prompt, "hi");
    assert_eq!(helper.chroot, conversation.chroot());
    assert!(calls.iter().all(|c| c.chroot == conversation.chroot()));
}

#[tokio::test]
async fn subagent_messages_carry_usage() {
    let Fixture {
        ctx,
        inference,
        tools,
        mut conversation,
        _chroot,
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
