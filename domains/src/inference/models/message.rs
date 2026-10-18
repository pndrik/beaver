// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::{Deserialize, Serialize};

use super::{Agent, Usage};
use crate::tools::models::Call;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Agent,
    System,
    User,
    Tool,
    /// Summary of the messages before it, see `Conversation::context`.
    Summary,
}

impl MessageType {
    pub fn as_string(&self) -> &'static str {
        match self {
            Self::Agent => "assistant",
            Self::System => "system",
            Self::User => "user",
            Self::Tool => "tool",
            Self::Summary => "summary",
        }
    }
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
pub enum Message {
    Agent(AgentMessage),
    Input(InputMessage),
}

#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub name: String,
    pub display_name: String,
    pub content: Option<String>,
    pub tool_calls: Vec<Call>,
    pub usage: Usage,
    pub cache_level: MessageCacheLevel,
}

#[derive(Debug, Clone)]
pub struct InputMessage {
    pub message_type: MessageType,
    pub content: String,
    pub cache_level: MessageCacheLevel,
}

impl Message {
    fn input(message_type: MessageType, content: String, cache_level: &MessageCacheLevel) -> Self {
        Self::Input(InputMessage {
            message_type,
            content,
            cache_level: cache_level.clone(),
        })
    }

    pub fn system(content: String, agent: &Agent) -> Self {
        let caching = &agent.inference.caching;
        let cache_level = caching.system.as_ref().unwrap_or(&caching.default);
        Self::input(MessageType::System, content, cache_level)
    }

    pub fn user(content: String, agent: &Agent) -> Self {
        Self::input(MessageType::User, content, &agent.inference.caching.default)
    }

    pub fn tool(content: String, agent: &Agent) -> Self {
        Self::input(MessageType::Tool, content, &agent.inference.caching.default)
    }

    pub fn summary(content: String, agent: &Agent) -> Self {
        Self::input(
            MessageType::Summary,
            content,
            &agent.inference.caching.default,
        )
    }

    pub fn agent(
        agent: &Agent,
        content: Option<String>,
        tool_calls: Vec<Call>,
        usage: Usage,
    ) -> Self {
        Self::Agent(AgentMessage {
            name: agent.metadata.name.clone(),
            display_name: agent.metadata.display_name.clone(),
            content,
            tool_calls,
            usage,
            cache_level: agent.inference.caching.default.clone(),
        })
    }

    pub fn content(&self) -> Option<&str> {
        match self {
            Self::Agent(message) => message.content.as_deref(),
            Self::Input(message) => Some(&message.content),
        }
    }

    pub fn cache_level(&self) -> &MessageCacheLevel {
        match self {
            Self::Agent(message) => &message.cache_level,
            Self::Input(message) => &message.cache_level,
        }
    }

    pub fn message_type(&self) -> MessageType {
        match self {
            Self::Agent(_) => MessageType::Agent,
            Self::Input(message) => message.message_type,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Agent(message) => &message.name,
            Self::Input(message) => message.message_type.as_string(),
        }
    }

    pub fn as_agent(&self) -> Option<&AgentMessage> {
        match self {
            Self::Agent(message) => Some(message),
            Self::Input(_) => None,
        }
    }

    pub fn is_agent(&self) -> bool {
        self.message_type() == MessageType::Agent
    }

    pub fn is_user(&self) -> bool {
        self.message_type() == MessageType::User
    }

    pub fn is_tool(&self) -> bool {
        self.message_type() == MessageType::Tool
    }

    pub fn is_summary(&self) -> bool {
        self.message_type() == MessageType::Summary
    }
}
