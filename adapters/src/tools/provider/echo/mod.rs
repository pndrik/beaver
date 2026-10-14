// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::models::Conversation,
    tools::{
        models::{Call, Schema, SchemaField, SchemaFieldType, Tool, ToolPermission},
        traits::ToolsProvider,
    },
};

const ECHO: &str = "echo";
const FAIL: &str = "fail";

/// Offline tools: `echo` returns `echo <text> @ <chroot>`, `fail` always fails.
pub struct EchoProvider {}

impl EchoProvider {
    pub fn new(_ctx: &AppContext) -> Self {
        Self {}
    }

    fn tool(name: &str) -> Tool {
        let mut schema = Schema::new(name, "");
        schema.add_property(
            "text",
            false,
            SchemaField::new(SchemaFieldType::String, "Text to echo.", None),
        );
        Tool::new(name.to_string(), String::new(), schema)
    }
}

#[async_trait]
impl ToolsProvider for EchoProvider {
    async fn reload(&mut self, _ctx: &AppContext) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(&self, _ctx: &AppContext) -> Result<Vec<Tool>, AppError> {
        Ok(vec![Self::tool(ECHO), Self::tool(FAIL)])
    }

    async fn get(&self, ctx: &AppContext, name: &str) -> Result<Tool, AppError> {
        match name {
            ECHO | FAIL => Ok(Self::tool(name)),
            _ => Err(app_error!(
                NotFound,
                "tool_not_found",
                &format!("Tool '{}' not found.", name),
                ctx.clone()
            )),
        }
    }

    async fn execute(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        _permissions: ToolPermission,
        input: &Call,
    ) -> Result<String, AppError> {
        if input.name == FAIL {
            return Err(app_error!(
                Internal,
                "tool_failed",
                "Tool 'fail' failed on purpose.",
                ctx.clone()
            ));
        }

        let text = input
            .get_argument("text")
            .and_then(|value| value.as_string())
            .unwrap_or_default();
        Ok(format!("echo {} @ {}", text, conversation.chroot()))
    }
}
