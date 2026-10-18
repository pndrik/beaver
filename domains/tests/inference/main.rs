// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

//! Tests for the inference domain, wired with offline adapters.

mod support;

use app_domains::inference::models::{AgentInference, Compression, Message, Model, Options};
use support::{compression, fixture, fixture_with_options};

fn summaries(messages: &[Message]) -> Vec<&str> {
    messages
        .iter()
        .filter(|message| message.is_summary())
        .filter_map(|message| message.content())
        .collect()
}

#[tokio::test]
async fn every_response_is_an_agent_message_with_usage() {
    let mut fixture = fixture(Compression::default()).await;
    fixture.say("call:echo hi").await;

    let messages = fixture.conversation.messages();
    assert_eq!(messages.len(), 4);

    let call = messages[1].as_agent().unwrap();
    assert_eq!(call.name, "coder");
    assert_eq!(call.content, None);
    assert_eq!(call.tool_calls.len(), 1);
    assert_eq!(call.tool_calls[0].name, "echo");
    let usage = call.usage;
    assert!(usage.input > 0);
    assert_eq!(usage.output, 0);

    assert!(messages[2].is_tool());
    assert!(messages[2].content().unwrap().contains("echo hi @"));

    let reply = messages[3].as_agent().unwrap();
    assert_eq!(reply.content.as_deref(), Some("reply 1: call:echo hi"));
    assert!(reply.tool_calls.is_empty());
    let usage = reply.usage;
    assert_eq!(usage.output, "reply 1: call:echo hi".len() as u64);
    assert_eq!(fixture.conversation.tokens(), usage.total());
}

#[tokio::test]
async fn disabled_compression_keeps_the_history() {
    let mut fixture = fixture(Compression {
        enabled: false,
        ..compression(Some(1), Some(1))
    })
    .await;
    fixture.say("one").await;
    fixture.say("two").await;
    fixture.say("three").await;

    assert!(summaries(fixture.conversation.messages()).is_empty());
    let calls = fixture.scripted.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(
        calls[2].history,
        ["user", "assistant", "user", "assistant", "user"]
    );
}

#[tokio::test]
async fn the_message_limit_inserts_a_summary_before_the_latest_message() {
    let mut fixture = fixture(compression(None, Some(3))).await;
    fixture.say("one").await;
    fixture.say("two").await;

    let messages = fixture.conversation.messages();
    assert_eq!(messages.len(), 5);
    assert!(messages[2].is_summary());
    assert_eq!(messages[3].content(), Some("two"));
    assert_eq!(fixture.conversation.context().len(), 3);

    let calls = fixture.scripted.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[1].history, ["user"]);
    assert_eq!(calls[1].prompt, "[user]: one\n\n[coder]: reply 1: one");
    assert_eq!(calls[2].history, ["summary", "user"]);
    assert_eq!(calls[2].prompt, "two");
}

#[tokio::test]
async fn the_token_limit_triggers_on_the_reported_usage() {
    let mut below = fixture(compression(Some(u64::MAX), None)).await;
    below.say("one").await;
    let tokens = below.conversation.tokens();
    below.say("two").await;
    assert!(summaries(below.conversation.messages()).is_empty());

    let mut fixture = fixture(compression(Some(tokens), None)).await;
    fixture.say("one").await;
    fixture.say("two").await;
    assert_eq!(summaries(fixture.conversation.messages()).len(), 1);
    assert_eq!(fixture.scripted.calls()[2].history, ["summary", "user"]);
}

#[tokio::test]
async fn tool_calls_are_part_of_the_summary() {
    let mut fixture = fixture(compression(None, Some(4))).await;
    fixture.say("call:echo hi").await;
    fixture.say("two").await;

    let calls = fixture.scripted.calls();
    let summarizer = &calls[2];
    assert!(
        summarizer
            .prompt
            .contains("[coder] called: echo({\"text\":\"hi\"})")
    );
    assert!(summarizer.prompt.contains("[tool]: Name: echo"));
}

#[tokio::test]
async fn a_new_summary_includes_the_previous_one() {
    let mut fixture = fixture(compression(None, Some(3))).await;
    fixture.say("one").await;
    fixture.say("two").await;
    fixture.say("three").await;

    let all = summaries(fixture.conversation.messages());
    assert_eq!(all.len(), 2);
    assert_eq!(fixture.conversation.messages().len(), 8);

    let calls = fixture.scripted.calls();
    let summarizer = &calls[3];
    assert!(
        summarizer
            .prompt
            .starts_with(&format!("[summary]: {}", all[0]))
    );
    assert_eq!(calls[4].history, ["summary", "user"]);
}

#[tokio::test]
async fn the_summarizer_uses_the_compression_model_and_prompt() {
    let mut fixture = fixture(Compression {
        model: Some(Model::ClaudeHaiku4_5),
        prompt: Some("Summarize!".to_string()),
        ..compression(None, Some(3))
    })
    .await;
    fixture.say("one").await;
    fixture.say("two").await;

    let calls = fixture.scripted.calls();
    assert_eq!(calls[0].model, Model::ClaudeOpus5);
    assert_eq!(calls[1].model, Model::ClaudeHaiku4_5);
    assert_eq!(calls[1].system_prompt, "Summarize!");
    assert_eq!(calls[2].model, Model::ClaudeOpus5);
}

#[tokio::test]
async fn the_summarizer_defaults_to_the_agent_model_and_builtin_prompt() {
    let mut fixture = fixture(compression(None, Some(3))).await;
    fixture.say("one").await;
    fixture.say("two").await;

    let calls = fixture.scripted.calls();
    assert_eq!(calls[1].model, Model::ClaudeOpus5);
    assert!(
        calls[1]
            .system_prompt
            .contains("You compress a conversation")
    );
}

#[tokio::test]
async fn insert_message_counts_negative_positions_from_the_end() {
    let mut fixture = fixture(Compression::default()).await;
    let conversation = &mut fixture.conversation;
    let mut insert = |content: &str, position: Option<i64>| {
        let message = Message::user(content.to_string(), conversation.agent());
        conversation.insert_message(message, position);
    };
    insert("b", None);
    insert("d", Some(-1));
    insert("c", Some(-2));
    insert("a", Some(0));
    insert("e", Some(100));
    insert("first", Some(-100));

    let contents: Vec<_> = fixture
        .conversation
        .messages()
        .iter()
        .filter_map(|message| message.content())
        .collect();
    assert_eq!(contents, ["first", "a", "b", "c", "d", "e"]);
}

#[tokio::test]
async fn the_agent_and_the_summarizer_use_the_agent_options() {
    let options = Options {
        max_tokens: 123,
        ..Options::default()
    };
    let mut fixture = fixture_with_options(compression(None, Some(3)), options).await;
    fixture.say("one").await;
    fixture.say("two").await;

    let calls = fixture.scripted.calls();
    assert_eq!(calls.len(), 3);
    assert!(calls.iter().all(|call| call.options.max_tokens == 123));
}

#[test]
fn options_can_be_set_partially() {
    let inference: AgentInference = yaml_serde::from_str(
        "model: claude-haiku-4-5\noptions:\n  max_tokens: 123\n  reasoning_effort: low\n",
    )
    .unwrap();
    assert_eq!(inference.options.max_tokens, 123);
    assert_eq!(inference.options.stop_sequences, Vec::<String>::new());
}
