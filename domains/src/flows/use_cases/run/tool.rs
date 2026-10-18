// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::collections::HashMap;

use serde_json::Value;

use super::{RunContext, values};
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    flows::{Flows, models::ArgumentsTool},
    inference::models::Message,
    tools::{
        Tools,
        models::{Call, CallValue, ToolPermission},
    },
};

impl Flows {
    pub(super) async fn tool(
        &self,
        ctx: &AppContext,
        context: &mut RunContext,
        arguments: &ArgumentsTool,
        values: &Value,
        tools: &Tools,
    ) -> Result<String, AppError> {
        let rendered = values::render_arguments(
            self.template_engine.as_ref(),
            ctx,
            &arguments.arguments,
            values,
        )
        .await?;
        let call_arguments: HashMap<String, CallValue> = match rendered {
            Value::Null => HashMap::new(),
            rendered => serde_json::from_value(rendered).map_err(|e| {
                app_error!(
                    Validation,
                    "tool_arguments_invalid",
                    &format!("Invalid arguments for tool '{}': {}", arguments.tool, e),
                    ctx.clone()
                )
            })?,
        };

        let call = Call::new(arguments.tool.clone(), call_arguments);
        let permissions = ToolPermission {
            name: arguments.tool.clone(),
            scopes: arguments.scopes.clone(),
        };
        let output = tools
            .execute_direct(ctx, &mut context.conversation, permissions, &call)
            .await?;

        context.conversation.insert_message(
            Message::tool(
                format!(
                    "Name: {}\nArguments: {}\nResult: \n{}",
                    call.name,
                    call.arguments_as_json(),
                    output
                ),
                context.conversation.agent(),
            ),
            None,
        );

        Ok(output)
    }
}
