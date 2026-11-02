// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{context::ContextStandard, file_system};
use std::sync::Arc;

use app_domains::{
    core::models::{AppContext, file::Chroot},
    inference::models::{
        Agent, AgentInference, CacheLevel, Compression, Conversation, Message, Model, Options,
        ReasoningEffort,
    },
};
use genai::chat::ChatRole;

use super::{GenAi, utils::with_trailing_slash};
use crate::configuration::ConfigurationMap;

fn context() -> AppContext {
    AppContext::new(Arc::new(ContextStandard::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
    )))
}

fn max_tokens(model: &Model, max_tokens: u64, reasoning_effort: ReasoningEffort) -> Option<u32> {
    let ctx = context();
    let options = Options {
        max_tokens,
        reasoning_effort,
        ..Options::default()
    };
    GenAi::get_options(&ctx, model, &options)
        .unwrap()
        .max_tokens
}

#[test]
fn max_tokens_stay_above_the_thinking_budget() {
    assert_eq!(
        max_tokens(&Model::ClaudeHaiku4_5, 4096, ReasoningEffort::Medium),
        Some(12096)
    );
    assert_eq!(
        max_tokens(&Model::ClaudeHaiku4_5, 4096, ReasoningEffort::High),
        Some(28096)
    );
}

#[test]
fn max_tokens_above_the_thinking_budget_are_kept() {
    assert_eq!(
        max_tokens(&Model::ClaudeHaiku4_5, 4096, ReasoningEffort::Low),
        Some(4096)
    );
    assert_eq!(
        max_tokens(&Model::ClaudeHaiku4_5, 10000, ReasoningEffort::Medium),
        Some(10000)
    );
}

#[test]
fn models_without_thinking_budget_keep_max_tokens() {
    assert_eq!(
        max_tokens(&Model::ClaudeOpus4_8, 4096, ReasoningEffort::High),
        Some(4096)
    );
    assert_eq!(
        max_tokens(&Model::ClaudeOpus4_5, 4096, ReasoningEffort::High),
        Some(4096)
    );
    assert_eq!(
        max_tokens(&Model::GPT5_5, 4096, ReasoningEffort::High),
        Some(4096)
    );
}

#[tokio::test]
async fn the_system_prompt_is_sent_after_a_summary() {
    let ctx = context();
    let agent = Agent::new(
        "coder".to_string(),
        "Coder".to_string(),
        String::new(),
        "You help.".to_string(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: CacheLevel::default(),
            options: Options::default(),
            compression: Compression::default(),
        },
    );
    let system_prompt = Message::system("You are a coder.".to_string(), &agent);
    let memory = Arc::new(file_system::Memory::new());
    let chroot = Chroot::new(&ctx, memory, "/", false).await.unwrap();
    let mut conversation = Conversation::new(system_prompt, agent, vec![], chroot);
    for content in ["one", "two"] {
        let message = Message::user(content.to_string(), conversation.agent());
        conversation.insert_message(message, None);
    }
    let summary = Message::summary("the user said one".to_string(), conversation.agent());
    conversation.insert_message(summary, Some(-2));

    let request = GenAi::get_chat_request(&conversation);
    let texts: Vec<_> = request
        .messages
        .iter()
        .map(|message| {
            (
                message.role.clone(),
                message.content.joined_texts().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        texts,
        [
            (ChatRole::System, "You are a coder.".to_string()),
            (
                ChatRole::User,
                "<conversation_summary>\nthe user said one\n</conversation_summary>".to_string()
            ),
            (ChatRole::User, "two".to_string()),
        ]
    );
}

#[test]
fn endpoints_get_a_trailing_slash() {
    assert_eq!(
        with_trailing_slash("https://api.openai.com/v1".to_string()),
        "https://api.openai.com/v1/"
    );
    assert_eq!(
        with_trailing_slash("https://api.openai.com/v1/".to_string()),
        "https://api.openai.com/v1/"
    );
}
