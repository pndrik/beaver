// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::{
    agent, configuration::ConfigurationMap, inference::Scripted, logger::LoggerRecording,
    tools::provider::EchoProvider,
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
    tools::{Tools, models::ToolPermission},
};

pub fn context() -> AppContext {
    AppContext::new(
        "test-trace".to_string(),
        Arc::new(ConfigurationMap::default()),
        Arc::new(LoggerRecording::new()),
    )
}

pub fn compression(tokens: Option<u64>, messages: Option<u64>) -> Compression {
    Compression {
        enabled: true,
        model: None,
        prompt: None,
        tokens,
        messages,
    }
}

fn agent(compression: Compression, options: Options) -> Agent {
    let mut agent = Agent::new(
        "coder".to_string(),
        "Coder".to_string(),
        String::new(),
        String::new(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: AgentCacheLevel::default(),
            options,
            compression,
        },
    );
    agent.tools.permissions = vec![ToolPermission {
        name: "echo".to_string(),
        scopes: vec![],
    }];
    agent
}

pub struct Fixture {
    pub ctx: AppContext,
    pub inference: Inference,
    pub tools: Tools,
    pub scripted: Arc<Scripted>,
    pub conversation: Conversation,
}

impl Fixture {
    /// Adds `prompt` as user message and runs inference.
    pub async fn say(&mut self, prompt: &str) {
        self.conversation.insert_message(
            Message::user(prompt.to_string(), self.conversation.agent()),
            None,
        );
        self.inference
            .infer(&self.ctx, &mut self.conversation, &self.tools)
            .await
            .unwrap();
    }
}

pub async fn fixture(compression: Compression) -> Fixture {
    fixture_with_options(compression, Options::default()).await
}

/// A conversation with the agent `coder`, answered by the scripted provider
/// and allowed to call `echo`.
pub async fn fixture_with_options(compression: Compression, options: Options) -> Fixture {
    let ctx = context();
    let scripted = Arc::new(Scripted::new());
    let inference = Inference::new(
        Arc::new(agent::Memory::new(vec![agent(compression, options)])),
        vec![scripted.clone()],
    )
    .unwrap();
    let tools = Tools::new(vec![Arc::new(EchoProvider::new(&ctx))]).unwrap();
    let conversation = inference
        .new_conversation(
            &ctx,
            "coder",
            &tools,
            &std::env::temp_dir().to_string_lossy(),
        )
        .await
        .unwrap();

    Fixture {
        ctx,
        inference,
        tools,
        scripted,
        conversation,
    }
}
