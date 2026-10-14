// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::path::Path;

use super::Message;
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::models::Agent,
    tools::models::Tool,
};

type Callback = Box<dyn Fn(Message) + Send + Sync>;

pub struct Conversation {
    system_prompt: Message,

    pub agent: Agent,
    pub tools: Vec<Tool>,

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

    pub fn chroot(&self) -> &str {
        &self.chroot
    }

    /// Resolves `path` inside the chroot. Fails if it doesn't exist or escapes the chroot.
    pub fn get_absolute_path(&self, ctx: &AppContext, path: &str) -> Result<String, AppError> {
        let joined = format!(
            "{}/{}",
            self.chroot.trim_end_matches('/'),
            path.trim_start_matches('/')
        );

        let canonicalized = Path::new(&joined).canonicalize().map_err(|e| {
            app_error!(
                Validation,
                "access_violation",
                &format!("Invalid path '{}': {}", path, e),
                ctx.clone()
            )
        })?;

        let chroot_canonicalized = Path::new(&self.chroot).canonicalize().map_err(|e| {
            app_error!(
                Validation,
                "access_violation",
                &format!("Invalid chroot '{}': {}", self.chroot, e),
                ctx.clone()
            )
        })?;
        if !canonicalized.starts_with(&chroot_canonicalized) {
            return Err(app_error!(
                Validation,
                "access_violation",
                &format!("Path '{}' is outside of chroot", path),
                ctx.clone()
            ));
        }

        if !canonicalized.exists() {
            return Err(app_error!(
                Validation,
                "access_violation",
                &format!("Path '{}' does not exist", path),
                ctx.clone()
            ));
        }

        Ok(canonicalized.to_string_lossy().to_string())
    }

    pub fn prompt(&self) -> &Message {
        &self.system_prompt
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message.clone());

        for subscriber in self.subscribers.iter() {
            subscriber(message.clone());
        }
    }

    pub fn add_assistant_message(&mut self, name: String, display_name: String, content: String) {
        self.add_message(Message::assistant(
            name,
            display_name,
            content,
            &self.agent.inference.caching.default,
        ));
    }
    pub fn add_user_message(&mut self, content: String) {
        self.add_message(Message::user(
            content,
            &self.agent.inference.caching.default,
        ));
    }
    pub fn add_tool_message(&mut self, content: String) {
        self.add_message(Message::tool(
            content,
            &self.agent.inference.caching.default,
        ));
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn get_latest_message(&self) -> Option<&Message> {
        self.messages.last()
    }
    pub fn get_latest_message_content(&self) -> Option<String> {
        self.get_latest_message().map(|m| m.content.clone())
    }
    pub fn append_to_latest_message(&mut self, content: String) {
        if let Some(last_message) = self.messages.last_mut() {
            last_message.content.push_str(&content);
        }
    }
    pub fn remove_latest_message(&mut self) {
        self.messages.pop();
    }

    pub fn subscribe(&mut self, callback: Callback) {
        self.subscribers.push(callback);
    }

    pub fn add_tool(&mut self, tool: Tool) {
        self.remove_tool(&tool.name);
        self.tools.push(tool);
    }

    pub fn remove_tool(&mut self, tool_name: &str) {
        self.tools.retain(|tool| tool.name != tool_name);
    }
}
