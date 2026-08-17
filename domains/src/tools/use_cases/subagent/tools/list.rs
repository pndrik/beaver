// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{Inference, models::Conversation},
    tools::{
        Tools,
        models::{Call, Schema, ToolPermission},
        use_cases::subagent::{self, traits::SubagentTool},
    },
};

const DESCRIPTION: &str = "Lists the subagents available to talk to.";

pub(in crate::tools::use_cases::subagent) struct List;

#[async_trait]
impl SubagentTool for List {
    fn name(&self) -> &'static str {
        subagent::LIST
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    async fn schema(&self, _ctx: &AppContext) -> Result<Schema, AppError> {
        Ok(Schema::new(subagent::LIST, DESCRIPTION))
    }

    async fn call(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
        permissions: ToolPermission,
        _input: &Call,
        _tools: &Tools,
        inference: &Inference,
    ) -> Result<(), AppError> {
        if !permissions.scopes.contains(&"_list".to_string()) {
            return Err(app_error!(
                Unauthorized,
                "tool_failed",
                "Can not list subagents without scope '_list'.",
                ctx.clone()
            ));
        }

        let subagents = inference
            .agent_provider
            .list(ctx)
            .await?
            .iter()
            .filter(|a| a.metadata.name != conversation.agent.metadata.name)
            .filter(|a| permissions.scopes.contains(&a.metadata.name))
            .map(|a| {
                format!(
                    "Name: {}\nDisplay Name: {}\nDescription: {}",
                    a.metadata.name, a.metadata.display_name, a.metadata.description
                )
            })
            .collect::<Vec<String>>();
        conversation.add_tool_message(format!("Available subagents:\n{}", subagents.join("\n\n")));

        Ok(())
    }
}
