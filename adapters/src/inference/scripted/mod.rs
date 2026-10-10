// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::sync::Mutex;

use app_domains::{
    core::models::{AppContext, AppError},
    inference::{
        models::{Conversation, Model, Options},
        traits::InferenceProvider,
    },
    tools::models::Call,
};

/// What the scripted provider saw on one inference call.
#[derive(Debug, Clone)]
pub struct ScriptedCall {
    pub agent: String,
    pub chroot: String,
    /// The latest user message.
    pub prompt: String,
    /// Message types of the conversation, e.g. `["user", "assistant", "tool", "user"]`.
    pub history: Vec<String>,
}

/// Offline inference provider for every model: answers each prompt with
/// `reply <n>: <prompt>`, where n counts the user messages so far, and never
/// calls tools. Every call is recorded.
#[derive(Debug, Default)]
pub struct Scripted {
    calls: Mutex<Vec<ScriptedCall>>,
}

impl Scripted {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn calls(&self) -> Vec<ScriptedCall> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl InferenceProvider for Scripted {
    async fn infer(
        &self,
        ctx: &AppContext,
        _options: &Options,
        conversation: &mut Conversation,
    ) -> Result<Vec<Call>, AppError> {
        let messages = conversation.messages();
        let prompt = messages
            .iter()
            .rev()
            .find(|message| message.is_user())
            .map(|message| message.content.clone())
            .unwrap_or_default();
        let user_messages = messages.iter().filter(|message| message.is_user()).count();
        let history = messages
            .iter()
            .map(|message| format!("{:?}", message.message_type).to_lowercase())
            .collect();

        let agent = conversation.agent.metadata.name.clone();
        self.calls.lock().unwrap().push(ScriptedCall {
            agent: agent.clone(),
            chroot: ctx.chroot.clone(),
            prompt: prompt.clone(),
            history,
        });

        let display_name = conversation.agent.metadata.display_name.clone();
        conversation.add_assistant_message(
            agent,
            display_name,
            format!("reply {}: {}", user_messages, prompt),
        );

        Ok(vec![])
    }

    async fn supported_models(&self, _ctx: &AppContext) -> Result<Vec<Model>, AppError> {
        Ok(vec![])
    }

    async fn supports_model(&self, _ctx: &AppContext, _model: &Model) -> Result<bool, AppError> {
        Ok(true)
    }
}
