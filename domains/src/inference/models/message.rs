// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    System,
    Assistant,
    User,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageContentType {
    Text,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub enum MessageCacheLevel {
    #[default]
    #[serde(rename = "none")]
    None,
    #[serde(rename = "5min")]
    Ephemeral5min,
    #[serde(rename = "1h")]
    Ephemeral1h,
    #[serde(rename = "24h")]
    Ephemeral24h,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub message_type: MessageType,
    pub content_type: MessageContentType,
    pub agent_name: String,
    pub agent_display_name: String,
    pub content: String,
    pub cache_level: MessageCacheLevel,
}

impl Message {
    pub fn new(
        message_type: MessageType,
        agent_name: String,
        agent_display_name: String,
        content: String,
        cache_level: MessageCacheLevel,
    ) -> Self {
        Self {
            message_type,
            content_type: MessageContentType::Text,
            agent_name,
            agent_display_name,
            content,
            cache_level,
        }
    }

    pub fn system(content: String, cache_level: &MessageCacheLevel) -> Self {
        Self::new(
            MessageType::System,
            "".to_string(),
            "".to_string(),
            content,
            cache_level.clone(),
        )
    }

    pub fn assistant(
        name: String,
        display_name: String,
        content: String,
        cache_level: &MessageCacheLevel,
    ) -> Self {
        Self::new(
            MessageType::Assistant,
            name,
            display_name,
            content,
            cache_level.clone(),
        )
    }

    pub fn user(content: String, cache_level: &MessageCacheLevel) -> Self {
        Self::new(
            MessageType::User,
            "user".to_string(),
            "User".to_string(),
            content,
            cache_level.clone(),
        )
    }

    pub fn tool(content: String, cache_level: &MessageCacheLevel) -> Self {
        Self::new(
            MessageType::Tool,
            "tool".to_string(),
            "Tool".to_string(),
            content,
            cache_level.clone(),
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
