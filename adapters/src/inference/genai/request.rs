// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use genai::chat::{ChatMessage, ChatRequest};

use super::{GenAi, utils::convert_cache_control};
use app_domains::inference::models::{Conversation, Message, MessageType};

fn new_message(conversation: &Conversation, internal_message: &Message) -> ChatMessage {
    match internal_message.message_type {
        MessageType::System => ChatMessage::system(internal_message.content.clone()),
        MessageType::Assistant => {
            if internal_message.agent_name != conversation.agent.metadata.name {
                ChatMessage::user(format!(
                    "[subagent/{}]: {}",
                    internal_message.agent_name, internal_message.content
                ))
            } else {
                ChatMessage::assistant(internal_message.content.clone())
            }
        }
        MessageType::User => ChatMessage::user(internal_message.content.clone()),
        MessageType::Tool => ChatMessage::user(
            "--- TOOL/SKILL RESULT ---\n".to_string()
                + &internal_message.content
                + "\n--- END TOOL/SKILL RESULT ---",
        ), // Many models either don't support tool messages or treat them not in a way we want them to treat them.
    }
}

impl GenAi {
    pub(super) fn get_chat_request(&self, conversation: &Conversation) -> ChatRequest {
        let mut chat_req = ChatRequest::default();

        let prompt = conversation.prompt();
        let mut system_message = new_message(conversation, prompt);
        if let Some(cache_level) = convert_cache_control(&prompt.cache_level) {
            system_message.options = Some(cache_level.into());
        }

        let mut messages: Vec<ChatMessage> = conversation
            .messages()
            .iter()
            .map(|m| new_message(conversation, m))
            .collect();

        if let Some(cache_level) =
            convert_cache_control(&conversation.agent.inference.caching.default)
            && let Some(last) = messages.last_mut()
        {
            last.options = Some(cache_level.into());
        }

        chat_req = chat_req.append_message(system_message);
        chat_req.append_messages(messages)
    }
}
