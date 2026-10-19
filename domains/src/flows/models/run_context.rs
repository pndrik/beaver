// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Value, json};

use crate::inference::models::Conversation;

/// State of a named context within one flow run.
pub(crate) struct RunContext {
    pub(crate) conversation: Conversation,
}

impl RunContext {
    /// The conversation as `Contexts.<context>` shows it to templates.
    pub(crate) fn messages(&self) -> Value {
        Value::Array(
            self.conversation
                .messages()
                .iter()
                .map(|message| {
                    let agent_message = message.as_agent();
                    json!({
                        "type": message.message_type().as_string(),
                        "agent": message.name(),
                        "content": message.content(),
                        "tool_calls": agent_message.map_or(&[][..], |m| &m.tool_calls),
                        "usage": agent_message.map(|m| m.usage),
                    })
                })
                .collect(),
        )
    }
}
