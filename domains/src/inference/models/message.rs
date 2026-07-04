// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    Assistant,
    User,
    Tool,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub message_type: MessageType,
    pub name: String,
    pub display_name: String,
    pub content: String,
}

impl Message {
    pub fn new(
        message_type: MessageType,
        name: String,
        display_name: String,
        content: String,
    ) -> Self {
        Self {
            message_type,
            name,
            display_name,
            content,
        }
    }

    pub fn assistant(name: String, display_name: String, content: String) -> Self {
        Self::new(MessageType::Assistant, name, display_name, content)
    }

    pub fn user(content: String) -> Self {
        Self::new(
            MessageType::User,
            "user".to_string(),
            "User".to_string(),
            content,
        )
    }

    pub fn tool(content: String) -> Self {
        Self::new(
            MessageType::Tool,
            "tool".to_string(),
            "Tool".to_string(),
            content,
        )
    }

    pub fn is_assistant(&self) -> bool {
        matches!(self.message_type, MessageType::Assistant)
    }

    pub fn is_user(&self) -> bool {
        matches!(self.message_type, MessageType::User)
    }

    pub fn is_tool(&self) -> bool {
        matches!(self.message_type, MessageType::Tool)
    }
}
