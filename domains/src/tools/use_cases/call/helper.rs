// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use tracing::{info, warn};

use super::REDIRECT_ARGUMENT;
use crate::{
    app_error,
    core::models::{
        AppContext, AppError,
        file::{self, Chroot},
    },
    inference::models::{Agent, Conversation, Message},
    tools::models::{Call, CallValue, Tool, ToolPermission},
};

pub(super) fn permission(
    ctx: &AppContext,
    agent: &Agent,
    call: &Call,
) -> Result<ToolPermission, AppError> {
    agent
        .tools
        .get_permission_for_tool(&call.name)
        .ok_or_else(|| {
            warn!(
                channel = "audit",
                object = "tool",
                action = "call_denied",
                agent = %agent.metadata.name,
                tool = %call.name,
                "No permission to access tool"
            );
            app_error!(
                Unauthorized,
                "tool_permission_denied",
                &format!("Agent does not have permission to call tool: {}", call.name),
                ctx.clone()
            )
        })
}

pub(super) fn redirect_allowed(
    ctx: &AppContext,
    agent: &Agent,
    call: &Call,
) -> Result<(), AppError> {
    if agent.tools.redirect_output {
        return Ok(());
    }

    info!(
        channel = "audit",
        object = "tool",
        action = "call_redirect_denied",
        agent = agent.metadata.name,
        tool = call.name
    );
    Err(app_error!(
        Forbidden,
        "access_violation",
        "Output redirection is not enabled for this agent",
        ctx.clone()
    ))
}

pub(super) fn validate_arguments(
    ctx: &AppContext,
    tool: &Tool,
    call: &Call,
) -> Result<(), AppError> {
    if call.validate_arguments(tool.schema.clone()) {
        return Ok(());
    }

    Err(app_error!(
        Validation,
        "tool_arguments_invalid",
        &format!(
            "Arguments for tool '{}' are invalid please check schema",
            call.name
        ),
        ctx.clone()
    ))
}

pub(super) fn not_found(ctx: &AppContext, call: &Call) -> AppError {
    app_error!(
        NotFound,
        "tool_not_found",
        &format!("Tool with name '{}' not found", call.name),
        ctx.clone()
    )
}

pub(super) fn get_redirect(call: &Call) -> Option<String> {
    match call.arguments.get(REDIRECT_ARGUMENT) {
        Some(CallValue::String(path)) => Some(path.clone()),
        _ => None,
    }
}

pub(super) fn result_message(call: &Call, output: &str) -> String {
    format!(
        "Name: {}\nArguments: {}\nResult: \n{}",
        call.name,
        call.arguments_as_json(),
        output
    )
}

pub(super) fn append_message(conversation: &mut Conversation, message: String) {
    let message = Message::tool(message, conversation.agent());
    conversation.insert_message(message, None);
}

pub(super) fn append_error(conversation: &mut Conversation, call: &Call, err: &AppError) {
    warn!(
        agent = %conversation.agent().metadata.name,
        tool = %call.name,
        error = %err.internal_message,
        "Tool failed"
    );
    let message = format!(
        "Name: {}\nArguments: {}\nError: \n{}",
        call.name,
        call.arguments_as_json(),
        err.internal_message
    );
    append_message(conversation, message);
}

pub(super) async fn write_to_file(
    ctx: &AppContext,
    call: &Call,
    chroot: &Chroot,
    path: &str,
    output: &str,
) -> Result<String, AppError> {
    chroot
        .write(ctx, path, output.as_bytes(), file::WriteMode::Truncate)
        .await?;
    Ok(format!(
        "Name: {}\nArguments: {}\nOutput written to '{}' ({} bytes, {} lines)",
        call.name,
        call.arguments_as_json(),
        path,
        output.len(),
        output.lines().count()
    ))
}
