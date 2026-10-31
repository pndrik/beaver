// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value};
use std::sync::Arc;

use app_adapters::{
    agent,
    configuration::ConfigurationMap,
    context::ContextStandard,
    flows,
    inference::Scripted,
    logger::LoggerRecording,
    template_engine::MiniJinjaTemplateEngine,
    tools::{output_writer, provider::EchoProvider},
};
use app_domains::{
    core::{
        models::AppContext,
        traits::{Configuration, Context, Logger},
    },
    flows::{Flows, models::Definition},
    inference::{
        Inference,
        models::{Agent, AgentCacheLevel, AgentInference, Compression, Model, Options},
    },
    tools::Tools,
};

pub const TRACE_ID: &str = "test-trace";

pub fn context(chroot: &str) -> (AppContext, Arc<LoggerRecording>) {
    let logger = Arc::new(LoggerRecording::new());
    let configuration: ConfigurationMap = [("context.chroot", chroot)].into_iter().collect();
    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        TRACE_ID.to_string(),
        Arc::new(configuration),
        logger.clone(),
    )));
    (ctx, logger)
}

pub const TENANT_TRACE_ID: &str = "tenant-trace";

/// A context whose public values hide the trace id and expose a tenant.
#[derive(Debug)]
struct TenantContext {
    configuration: Arc<dyn Configuration + Send + Sync>,
    logger: Arc<dyn Logger + Send + Sync>,
}

impl Context for TenantContext {
    fn trace_id(&self) -> &str {
        TENANT_TRACE_ID
    }

    fn configuration(&self) -> &Arc<dyn Configuration + Send + Sync> {
        &self.configuration
    }

    fn logger(&self) -> &Arc<dyn Logger + Send + Sync> {
        &self.logger
    }

    fn template_values(&self) -> Map<String, Value> {
        Map::from_iter([("tenant".to_string(), Value::from("acme"))])
    }
}

pub fn tenant_context() -> (AppContext, Arc<LoggerRecording>) {
    let logger = Arc::new(LoggerRecording::new());
    let configuration: ConfigurationMap = [(
        "context.chroot",
        std::env::temp_dir().to_string_lossy().as_ref(),
    )]
    .into_iter()
    .collect();
    let ctx = AppContext::new(Arc::new(TenantContext {
        configuration: Arc::new(configuration),
        logger: logger.clone(),
    }));
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
            compression: Compression::default(),
        },
    )
}

pub struct Fixture {
    pub flows: Flows,
    pub inference: Inference,
    pub tools: Tools,
    pub scripted: Arc<Scripted>,
}

/// Knows the agents `coder`, `calculator` and `dispatcher` (as used by the
/// examples), answers with the scripted provider and offers the
/// `echo` and `fail` tools.
pub fn fixture(ctx: &AppContext, flows: Vec<Definition>) -> Fixture {
    let scripted = Arc::new(Scripted::new());
    let inference = Inference::new(
        Arc::new(agent::Memory::new(vec![
            agent("coder"),
            agent("calculator"),
            agent("dispatcher"),
        ])),
        vec![scripted.clone()],
    )
    .unwrap();
    let flows = Flows::new(
        Arc::new(flows::Memory::new(flows)),
        Arc::new(MiniJinjaTemplateEngine::new(ctx)),
    )
    .unwrap();
    let tools = Tools::new(
        vec![Arc::new(EchoProvider::new(ctx))],
        Arc::new(output_writer::Memory::new()),
    )
    .unwrap();

    Fixture {
        flows,
        inference,
        tools,
        scripted,
    }
}
