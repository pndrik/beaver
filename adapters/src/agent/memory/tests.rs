// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_domains::{
    core::models::AppContext,
    inference::{
        models::{Agent, AgentInference, CacheLevel, Compression, Model, Options},
        traits::AgentProvider,
    },
};

use super::Memory;
use crate::{configuration::ConfigurationMap, context::ContextStandard};

fn context() -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )))
}

fn agent(name: &str, prompt: &str) -> Agent {
    Agent::new(
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
    )
}

#[tokio::test]
async fn returns_valid_agents() {
    let ctx = context();
    let memory = Memory::new(vec![agent("a", "You help.")]);

    assert_eq!(memory.get(&ctx, "a").await.unwrap().metadata.name, "a");
    assert_eq!(memory.list(&ctx).await.unwrap().len(), 1);
}

#[tokio::test]
async fn get_rejects_an_invalid_agent() {
    let ctx = context();
    let memory = Memory::new(vec![agent("a", " ")]);

    let err = memory.get(&ctx, "a").await.unwrap_err();
    assert_eq!(err.translation_key, "agent_invalid");
    assert_eq!(
        err.internal_message,
        "Agent 'a' is invalid: spec.prompt is empty"
    );
}

#[tokio::test]
async fn list_fails_if_one_agent_is_invalid() {
    let ctx = context();
    let mut denied = agent("b", "You help.");
    denied.spec.tools.deny = vec![app_domains::tools::models::ToolDenial {
        name: "^git_".to_string(),
    }];
    let memory = Memory::new(vec![agent("a", "You help."), denied]);

    let err = memory.list(&ctx).await.unwrap_err();
    assert_eq!(err.translation_key, "agent_invalid");
    assert_eq!(
        err.internal_message,
        "Agent 'b' is invalid: spec.tools.deny[0]: name '^git_' must end with '$'"
    );
}

#[tokio::test]
async fn get_of_an_unknown_agent_fails() {
    let ctx = context();
    let memory = Memory::new(vec![]);

    let err = memory.get(&ctx, "a").await.unwrap_err();
    assert_eq!(err.translation_key, "agent_not_found");
}
