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
        models::{Agent, AgentCacheLevel, AgentInference, Model, Options},
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
        },
    );
    agent.tools.permissions = permissions;
    agent
}

#[tokio::test]
async fn subagent_runs_in_the_chroot_of_its_parent() {
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

    let mut conversation = inference
        .new_conversation(&ctx, "parent", &tools, &chroot.path().to_string_lossy())
        .await
        .unwrap();
    let call = Call::new(
        "subagent_invoke".to_string(),
        HashMap::from([
            ("name".to_string(), CallValue::String("helper".to_string())),
            ("prompt".to_string(), CallValue::String("hi".to_string())),
        ]),
    );
    tools
        .call(&ctx, &mut conversation, &inference, &call)
        .await
        .unwrap();

    let calls = scripted.calls();
    let helper = calls.iter().find(|c| c.agent == "helper").unwrap();
    assert_eq!(helper.prompt, "hi");
    assert_eq!(helper.chroot, conversation.chroot());
    assert!(calls.iter().all(|c| c.chroot == conversation.chroot()));
}
