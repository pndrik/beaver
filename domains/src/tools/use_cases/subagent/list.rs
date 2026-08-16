// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{models::Conversation, traits::AgentProvider},
    tools::models::ToolPermission,
};

pub(super) async fn list(
    ctx: &AppContext,
    conversation: &mut Conversation,
    permissions: ToolPermission,
    subagent_provider: Arc<dyn AgentProvider + Send + Sync>,
) -> Result<(), AppError> {
    if !permissions.scopes.contains(&"list".to_string()) {
        return Err(app_error!(
            Unauthorized,
            "tool_failed",
            "Can not delete files without scope 'list'.",
            ctx.clone()
        ));
    }

    let subagents = subagent_provider
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
