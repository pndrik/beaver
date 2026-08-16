// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use super::Message;
use crate::inference::models::Agent;
use crate::tools::models::Tool;

type Callback = Box<dyn Fn(Message) + Send + Sync>;

pub struct Conversation {
    system_prompt: String,

    pub agent: Agent,
    pub tools: Vec<Tool>,

    messages: Vec<Message>,
    subscribers: Vec<Callback>,
}

impl Conversation {
    pub fn new(system_prompt: String, agent: Agent, tools: Vec<Tool>) -> Self {
        Self {
            system_prompt,
            agent,
            tools,
            messages: vec![],
            subscribers: vec![],
        }
    }

    pub fn prompt(&self) -> String {
        self.system_prompt.clone() + "\n\n---\n# Prompt\n" + &self.agent.prompt
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message.clone());

        for subscriber in self.subscribers.iter() {
            subscriber(message.clone());
        }
    }

    pub fn add_assistant_message(&mut self, name: String, display_name: String, content: String) {
        self.add_message(Message::assistant(name, display_name, content));
    }
    pub fn add_user_message(&mut self, content: String) {
        self.add_message(Message::user(content));
    }
    pub fn add_tool_message(&mut self, content: String) {
        self.add_message(Message::tool(content));
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
}
