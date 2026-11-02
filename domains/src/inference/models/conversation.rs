// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::collections::HashMap;

use tracing::debug;

use super::{Agent, ConversationValue, Message, Usage};
use crate::{core::models::file::Chroot, tools::models::Tool};

type Callback = Box<dyn Fn(Message) + Send + Sync>;

pub struct Conversation {
    system_prompt: Message,

    agent: Agent,
    tools: Vec<Tool>,

    chroot: Chroot,
    messages: Vec<Message>,
    subscribers: Vec<Callback>,
    values: HashMap<String, ConversationValue>,
}

impl Conversation {
    pub fn new(system_prompt: Message, agent: Agent, tools: Vec<Tool>, chroot: Chroot) -> Self {
        let mut conversation = Self {
            system_prompt,
            agent,
            tools: vec![],
            chroot,
            messages: vec![],
            subscribers: vec![],
            values: HashMap::new(),
        };

        for tool in tools {
            conversation.add_tool(tool);
        }

        conversation
    }

    /*
     * System prompt
     */
    pub fn system_prompt(&self) -> &Message {
        &self.system_prompt
    }

    /*
     * Agent
     */
    pub fn agent(&self) -> &Agent {
        &self.agent
    }

    /*
     * Tools
     */
    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    pub fn add_tool(&mut self, tool: Tool) {
        self.remove_tool(&tool.name);
        self.tools.push(tool);
    }

    pub fn remove_tool(&mut self, tool_name: &str) {
        self.tools.retain(|tool| tool.name != tool_name);
    }

    /*
     * Chroot
     */
    /// The files the conversation's tools work on.
    pub fn chroot(&self) -> &Chroot {
        &self.chroot
    }

    /*
     * Messages
     */
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn last(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub fn insert_message(&mut self, message: Message, position: Option<i64>) {
        let len = self.messages.len();
        let index = match position.unwrap_or(-1) {
            position if position < 0 => usize::try_from(position.unsigned_abs() - 1)
                .map_or(0, |from_end| len.saturating_sub(from_end)),
            position => usize::try_from(position).map_or(len, |position| position.min(len)),
        };
        debug!(
            channel = "conversation",
            agent = self.agent.metadata.name.as_str(),
            message_type = message.message_type().as_string(),
            content = message.content().unwrap_or_default(),
        );

        for subscriber in &self.subscribers {
            subscriber(message.clone());
        }

        self.messages.insert(index, message);
    }

    /// The messages sent to the model: from the latest summary onward, or all.
    pub fn context(&self) -> &[Message] {
        let start = self
            .messages
            .iter()
            .rposition(Message::is_summary)
            .unwrap_or(0);
        &self.messages[start..]
    }

    /// The usage the provider last reported for this agent since the latest
    /// summary, `None` if it has not responded since. Replies of subagents are
    /// skipped, they report the subagent's context.
    pub fn usage(&self) -> Option<Usage> {
        self.context()
            .iter()
            .rev()
            .filter_map(Message::as_agent)
            .find(|message| message.name == self.agent.metadata.name)
            .map(|message| message.usage)
    }

    /// The context size the provider last reported for this agent.
    pub fn tokens(&self) -> u64 {
        self.usage().map_or(0, |usage| usage.total())
    }

    pub fn needs_compression(&self) -> bool {
        let compression = &self.agent.spec.inference.compression;
        if !compression.enabled {
            return false;
        }

        let messages = self
            .context()
            .iter()
            .filter(|message| !message.is_summary())
            .count() as u64;

        // Fail safe if the conversation is too short to compress.
        if messages < 2 {
            return false;
        }

        compression
            .tokens
            .is_some_and(|max_tokens| self.tokens() >= max_tokens)
            || compression
                .messages
                .is_some_and(|max_messages| messages >= max_messages)
    }

    /*
     * Values
     */
    /// The value tools stored under `key`.
    pub fn value(&self, key: &str) -> Option<&ConversationValue> {
        self.values.get(key)
    }

    /// Stores `value` under `key`, replacing an existing one.
    pub fn set_value(&mut self, key: impl Into<String>, value: ConversationValue) {
        self.values.insert(key.into(), value);
    }

    /// Removes the value under `key` and returns it.
    pub fn remove_value(&mut self, key: &str) -> Option<ConversationValue> {
        self.values.remove(key)
    }

    /*
     * Subscribers
     */
    pub fn subscribe(&mut self, callback: Callback) {
        self.subscribers.push(callback);
    }
}
