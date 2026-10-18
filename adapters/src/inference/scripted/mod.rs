// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::{collections::HashMap, sync::Mutex};

use app_domains::{
    core::models::{AppContext, AppError},
    inference::models::{Conversation, Message, Model, Options, Usage},
    inference::traits::InferenceProvider,
    tools::models::{Call, CallValue},
};

/// What the scripted provider saw on one inference call.
#[derive(Debug, Clone)]
pub struct ScriptedCall {
    pub agent: String,
    pub model: Model,
    pub options: Options,
    pub system_prompt: String,
    pub chroot: String,
    /// The latest user message.
    pub prompt: String,
    /// Message names of the context, e.g. `["summary", "user", "assistant", "tool", "user"]`.
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

fn scripted_tool_call(message: Option<&Message>) -> Option<Call> {
    let message = message.filter(|message| message.is_user())?;
    let (name, text) = message.content()?.strip_prefix("call:")?.split_once(' ')?;
    Some(Call::new(
        name.to_string(),
        HashMap::from([("text".to_string(), CallValue::String(text.to_string()))]),
    ))
}

#[async_trait]
impl InferenceProvider for Scripted {
    async fn infer(
        &self,
        _ctx: &AppContext,
        options: &Options,
        conversation: &mut Conversation,
    ) -> Result<(), AppError> {
        let messages = conversation.messages();
        let prompt = messages
            .iter()
            .rev()
            .find(|message| message.is_user())
            .and_then(|message| message.content())
            .unwrap_or_default()
            .to_string();
        let user_messages = messages.iter().filter(|message| message.is_user()).count();
        let context = conversation.context();
        let history = context
            .iter()
            .map(|message| message.message_type().as_string().to_string())
            .collect();
        let system_prompt = conversation.system_prompt().content().unwrap_or_default();
        let input_tokens = std::iter::once(system_prompt)
            .chain(context.iter().filter_map(|message| message.content()))
            .map(|content| content.chars().count() as u64)
            .sum();

        self.calls.lock().unwrap().push(ScriptedCall {
            agent: conversation.agent().metadata.name.clone(),
            model: conversation.agent().model().clone(),
            options: options.clone(),
            system_prompt: system_prompt.to_string(),
            chroot: conversation.chroot().to_string(),
            prompt: prompt.clone(),
            history,
        });

        let (content, tool_calls) = match scripted_tool_call(conversation.last()) {
            Some(call) => (None, vec![call]),
            None => (Some(format!("reply {}: {}", user_messages, prompt)), vec![]),
        };
        let output_tokens = content.as_deref().map_or(0, |c| c.chars().count() as u64);
        conversation.insert_message(
            Message::agent(
                conversation.agent(),
                content,
                tool_calls,
                Usage {
                    input: input_tokens,
                    output: output_tokens,
                },
            ),
            None,
        );

        Ok(())
    }

    async fn supported_models(&self, _ctx: &AppContext) -> Result<Vec<Model>, AppError> {
        Ok(vec![])
    }

    async fn supports_model(&self, _ctx: &AppContext, _model: &Model) -> Result<bool, AppError> {
        Ok(true)
    }
}
