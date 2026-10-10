// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::{
    agent, configuration::ConfigurationMap, flows, inference::Scripted, logger::LoggerRecording,
    template_engine::MiniJinjaTemplateEngine, tools::provider::EchoProvider, webhooks,
};
use app_domains::{
    core::models::AppContext,
    inference::{
        Inference,
        models::{Agent, AgentCacheLevel, AgentInference, Flow, Model, Options, Webhook},
    },
    tools::Tools,
};

pub const TRACE_ID: &str = "test-trace";

pub fn context(chroot: &str) -> (AppContext, Arc<LoggerRecording>) {
    let logger = Arc::new(LoggerRecording::new());
    let ctx = AppContext::new(
        TRACE_ID.to_string(),
        chroot.to_string(),
        Arc::new(ConfigurationMap::default()),
        logger.clone(),
    )
    .unwrap();
    (ctx, logger)
}

pub fn temp_context() -> (AppContext, Arc<LoggerRecording>) {
    context(&std::env::temp_dir().to_string_lossy())
}

fn agent(name: &str) -> Agent {
    Agent::new(
        name.to_string(),
        name.to_string(),
        String::new(),
        String::new(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: AgentCacheLevel::default(),
            options: Options::default(),
        },
    )
}

pub struct Fixture {
    pub inference: Inference,
    pub tools: Tools,
    pub scripted: Arc<Scripted>,
}

/// Knows the agent `coder`, answers with the scripted provider and offers the
/// `echo` and `fail` tools.
pub fn fixture(ctx: &AppContext, webhooks: Vec<Webhook>, flows: Vec<Flow>) -> Fixture {
    let scripted = Arc::new(Scripted::new());
    let inference = Inference::new(
        Arc::new(agent::Memory::new(vec![agent("coder")])),
        vec![scripted.clone()],
        Arc::new(webhooks::Memory::new(webhooks)),
        Arc::new(flows::Memory::new(flows)),
        Arc::new(MiniJinjaTemplateEngine::new(ctx)),
    )
    .unwrap();
    let tools = Tools::new(vec![Arc::new(EchoProvider::new(ctx))]).unwrap();

    Fixture {
        inference,
        tools,
        scripted,
    }
}
