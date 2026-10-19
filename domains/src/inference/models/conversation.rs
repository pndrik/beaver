// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::path::Path;

use super::{Agent, Message, Usage, chroot};
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    tools::models::Tool,
};

type Callback = Box<dyn Fn(Message) + Send + Sync>;

pub struct Conversation {
    system_prompt: Message,

    agent: Agent,
    tools: Vec<Tool>,

    chroot: String,
    messages: Vec<Message>,
    subscribers: Vec<Callback>,
}

impl Conversation {
    pub fn new(
        ctx: &AppContext,
        system_prompt: Message,
        agent: Agent,
        tools: Vec<Tool>,
        chroot: &str,
    ) -> Result<Self, AppError> {
        let chroot = Path::new(chroot.trim_end_matches('/'))
            .canonicalize()
            .map_err(|e| {
                app_error!(
                    Validation,
                    "access_violation",
                    &format!("Invalid chroot '{}': {}", chroot, e),
                    ctx.clone()
                )
            })?
            .to_string_lossy()
            .to_string();

        let mut conversation = Self {
            system_prompt,
            agent,
            tools: vec![],
            chroot,
            messages: vec![],
            subscribers: vec![],
        };

        for tool in tools {
            conversation.add_tool(tool);
        }

        Ok(conversation)
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
    pub fn chroot(&self) -> &str {
        &self.chroot
    }

    pub fn get_absolute_path(&self, ctx: &AppContext, path: &str) -> Result<String, AppError> {
        chroot::resolve_path(ctx, &self.chroot, path)
    }

    pub fn get_new_path(&self, ctx: &AppContext, path: &str) -> Result<String, AppError> {
        chroot::resolve_new_path(ctx, &self.chroot, path)
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
        let len = self.messages.len() as i64;
        let index = match position.unwrap_or(-1) {
            position if position < 0 => len + 1 + position,
            position => position,
        };
        self.messages
            .insert(index.clamp(0, len) as usize, message.clone());

        for subscriber in self.subscribers.iter() {
            subscriber(message.clone());
        }
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
        let compression = &self.agent.inference.compression;
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
     * Subscribers
     */
    pub fn subscribe(&mut self, callback: Callback) {
        self.subscribers.push(callback);
    }
}
