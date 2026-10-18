// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::io::{self, Write};

use app::App;
use app_domains::{
    core::models::{AppContext, AppError},
    inference::models::{Message, MessageType},
};

pub async fn run_conversation(
    ctx: &AppContext,
    application: &App,
    agent: &str,
) -> Result<(), AppError> {
    let mut conversation = application
        .new_conversation(&ctx, agent)
        .await
        .expect("Failed to create new conversation");

    conversation.subscribe(Box::new(|message| {
        if let Message::Agent(agent_message) = &message
            && let Some(content) = &agent_message.content
        {
            println!("[{}]: {}", agent_message.display_name, content);
        }
    }));

    loop {
        print!("[You]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).unwrap_or(0) == 0 {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input == "/dump" {
            println!("--- Conversation Dump ---");
            for message in conversation.messages() {
                let agent_message = message.as_agent();
                let name = match message.message_type() {
                    MessageType::Agent => agent_message.map_or("", |m| &m.display_name),
                    MessageType::System => "System",
                    MessageType::User => "You",
                    MessageType::Tool => "Tool",
                    MessageType::Summary => "Summary",
                };
                let usage = agent_message
                    .map(|m| format!(" ({}/{} tokens)", m.usage.input, m.usage.output))
                    .unwrap_or_default();

                if let Some(content) = message.content() {
                    println!("> [{}]{}: {}", name, usage, content);
                }
                for call in agent_message.iter().flat_map(|m| &m.tool_calls) {
                    println!(
                        "> [{}]{} called: {}({})",
                        name,
                        usage,
                        call.name,
                        call.arguments_as_json()
                    );
                }
            }
            println!("--- End of Dump ---\n");
            continue;
        }

        conversation.insert_message(Message::user(input.to_string(), conversation.agent()), None);
        match application.infer(&ctx, &mut conversation).await {
            Ok(_) => {}
            Err(e) => {
                println!("Error during inference: {}", e.internal_message);
            }
        }
    }

    Ok(())
}
