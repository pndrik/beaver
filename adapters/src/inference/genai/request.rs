// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use genai::chat::{ChatMessage, ChatRequest};

use super::GenAi;
use app_domains::inference::models::{Conversation, MessageType};

impl GenAi {
    pub(super) fn get_chat_request(&self, conversation: &Conversation) -> ChatRequest {
        let mut chat_req = ChatRequest::default().with_system(conversation.prompt());

        for m in conversation.messages() {
            chat_req = chat_req.append_message(match m.message_type {
                MessageType::Assistant => {
                    if m.name != conversation.agent.metadata.name {
                        ChatMessage::user(format!(
                            "[subagent/{}]: {}",
                            m.display_name,
                            m.content.clone()
                        ))
                    } else {
                        ChatMessage::assistant(m.content.clone())
                    }
                }
                MessageType::User => ChatMessage::user(m.content.clone()),
                MessageType::Tool => ChatMessage::user(
                    "--- TOOL/SKILL RESULT ---\n".to_string()
                        + &m.content
                        + "\n--- END TOOL/SKILL RESULT ---",
                ), // Many models either don't support tool messages or treat them not in a way we want them to treat them.
            })
        }
        chat_req
    }
}
