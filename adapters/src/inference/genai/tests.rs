// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_domains::{
    core::models::AppContext,
    inference::models::{
        Agent, AgentCacheLevel, AgentInference, Compression, Conversation, Message, Model, Options,
        ReasoningEffort,
    },
};
use genai::chat::ChatRole;

use super::GenAi;
use crate::{configuration::ConfigurationMap, logger::LoggerRecording};

fn context() -> AppContext {
    AppContext::new(
        "test".to_string(),
        Arc::new(ConfigurationMap::default()),
        Arc::new(LoggerRecording::new()),
    )
}

fn max_tokens(model: Model, max_tokens: u64, reasoning_effort: ReasoningEffort) -> Option<u32> {
    let ctx = context();
    let options = Options {
        max_tokens,
        reasoning_effort,
        ..Options::default()
    };
    GenAi::new(&ctx, "endpoint", "apikey")
        .get_options(&ctx, &model, &options)
        .unwrap()
        .max_tokens
}

#[test]
fn max_tokens_stay_above_the_thinking_budget() {
    assert_eq!(
        max_tokens(Model::ClaudeHaiku4_5, 4096, ReasoningEffort::Medium),
        Some(12096)
    );
    assert_eq!(
        max_tokens(Model::ClaudeHaiku4_5, 4096, ReasoningEffort::High),
        Some(28096)
    );
}

#[test]
fn max_tokens_above_the_thinking_budget_are_kept() {
    assert_eq!(
        max_tokens(Model::ClaudeHaiku4_5, 4096, ReasoningEffort::Low),
        Some(4096)
    );
    assert_eq!(
        max_tokens(Model::ClaudeHaiku4_5, 10000, ReasoningEffort::Medium),
        Some(10000)
    );
}

#[test]
fn models_without_thinking_budget_keep_max_tokens() {
    assert_eq!(
        max_tokens(Model::ClaudeOpus4_8, 4096, ReasoningEffort::High),
        Some(4096)
    );
    assert_eq!(
        max_tokens(Model::ClaudeOpus4_5, 4096, ReasoningEffort::High),
        Some(4096)
    );
    assert_eq!(
        max_tokens(Model::GPT5_5, 4096, ReasoningEffort::High),
        Some(4096)
    );
}

#[test]
fn the_system_prompt_is_sent_after_a_summary() {
    let ctx = context();
    let agent = Agent::new(
        "coder".to_string(),
        "Coder".to_string(),
        String::new(),
        String::new(),
        AgentInference {
            model: Model::ClaudeOpus5,
            caching: AgentCacheLevel::default(),
            options: Options::default(),
            compression: Compression::default(),
        },
    );
    let system_prompt = Message::system("You are a coder.".to_string(), &agent);
    let chroot = std::env::temp_dir();
    let mut conversation = Conversation::new(
        &ctx,
        system_prompt,
        agent,
        vec![],
        &chroot.to_string_lossy(),
    )
    .unwrap();
    for content in ["one", "two"] {
        let message = Message::user(content.to_string(), conversation.agent());
        conversation.insert_message(message, None);
    }
    let summary = Message::summary("the user said one".to_string(), conversation.agent());
    conversation.insert_message(summary, Some(-2));

    let request = GenAi::new(&ctx, "endpoint", "apikey").get_chat_request(&conversation);
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
