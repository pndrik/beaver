// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value};
use std::sync::Arc;

use app_adapters::{
    agent, configuration::ConfigurationMap, context::ContextStandard, file_system, flows,
    inference::Scripted, logging::Memory, template_engine::MiniJinjaTemplateEngine,
    tools::provider::EchoProvider,
};
use app_domains::{
    core::{
        models::{AppContext, LogAttributes},
        traits::{Configuration, Context, Logging},
    },
    flows::{Flows, models::Definition},
    inference::{
        Inference,
        models::{Agent, AgentInference, CacheLevel, Compression, Model, Options},
    },
    tools::Tools,
};

pub const TRACE_ID: &str = "test-trace";

pub fn temp_context() -> (AppContext, Memory) {
    let ctx = AppContext::new(Arc::new(ContextStandard::new(
        TRACE_ID.to_string(),
        Arc::new(ConfigurationMap::default()),
    )));
    let logger = Memory::new();
    logger.init(&ctx, "trace").unwrap();
    (ctx, logger)
}

pub const TENANT_TRACE_ID: &str = "tenant-trace";

/// A context whose public values hide the trace id and expose a tenant.
#[derive(Debug)]
struct TenantContext {
    configuration: Arc<dyn Configuration + Send + Sync>,
}

impl Context for TenantContext {
    fn trace_id(&self) -> &str {
        TENANT_TRACE_ID
    }

    fn configuration(&self) -> &Arc<dyn Configuration + Send + Sync> {
        &self.configuration
    }

    fn template_values(&self) -> Map<String, Value> {
        Map::from_iter([("tenant".to_string(), Value::from("acme"))])
    }

    fn log_attributes(&self) -> LogAttributes {
        let mut attributes = LogAttributes::new(TENANT_TRACE_ID).with("tenant", "acme");
        attributes
            .custom
            .insert("trace_id".to_string(), Value::from("spoofed"));
        attributes
    }
}

pub fn tenant_context() -> (AppContext, Memory) {
    let ctx = AppContext::new(Arc::new(TenantContext {
        configuration: Arc::new(ConfigurationMap::default()),
    }));
    let logger = Memory::new();
    logger.init(&ctx, "trace").unwrap();
    (ctx, logger)
}

fn agent(name: &str) -> Agent {
    Agent::new(
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
    )
}

pub struct Fixture {
    pub flows: Flows,
    pub inference: Inference,
    pub tools: Tools,
    pub scripted: Arc<Scripted>,
    pub file_system: Arc<file_system::Memory>,
}

/// An agent that may call everything except `echo`.
fn guarded() -> Agent {
    let mut agent = agent("guarded");
    agent.spec.tools = yaml_serde::from_str("allow: [{ name: ^.*$ }]\ndeny: [{ name: echo }]")
        .expect("valid rules");
    agent
}

/// Knows the agents `coder`, `calculator` and `dispatcher` (as used by the
/// examples) and `guarded`, answers with the scripted provider and offers the
/// `echo` and `fail` tools. Files live in memory.
pub fn fixture(ctx: &AppContext, flows: Vec<Definition>) -> Fixture {
    let file_system = Arc::new(file_system::Memory::new());
    let scripted = Arc::new(Scripted::new());
    let inference = Inference::new(
        Arc::new(agent::Memory::new(vec![
            agent("coder"),
            agent("calculator"),
            agent("dispatcher"),
            guarded(),
        ])),
        vec![scripted.clone()],
    )
    .unwrap();
    let flows = Flows::new(
        Arc::new(flows::Memory::new(flows)),
        Arc::new(MiniJinjaTemplateEngine::new(ctx)),
        file_system.clone(),
    )
    .unwrap();
    let tools = Tools::new(vec![Arc::new(EchoProvider::new(ctx))]).unwrap();

    Fixture {
        flows,
        inference,
        tools,
        scripted,
        file_system,
    }
}
