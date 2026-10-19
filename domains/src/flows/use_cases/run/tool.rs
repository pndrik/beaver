// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::collections::HashMap;

use serde_json::Value;

use super::values;
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    flows::{
        Flows,
        models::{Action, FlowRun, Step},
    },
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
        step: &Step,
        tools: &Tools,
        run: &mut FlowRun,
    ) -> Result<(), AppError> {
        let Action::Tool(arguments) = &step.action else {
            unreachable!("tool runs tool steps")
        };
        let values = run.template_values(ctx);
        let rendered = values::render_arguments(
            self.template_engine.as_ref(),
            ctx,
            &arguments.arguments,
            &values,
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
        let conversation = &mut run.context_mut(step).conversation;
        let output = tools
            .execute_direct(ctx, conversation, permissions, &call)
            .await?;

        conversation.insert_message(
            Message::tool(
                format!(
                    "Name: {}\nArguments: {}\nResult: \n{}",
                    call.name,
                    call.arguments_as_json(),
                    output
                ),
                conversation.agent(),
            ),
            None,
        );

        Ok(())
    }
}
