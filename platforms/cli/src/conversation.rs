// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::io::{self, Write};

use app::App;
use app_domains::{
    core::models::{AppContext, AppError},
    inference::models::MessageType,
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
        if message.message_type != MessageType::Assistant {
            return;
        }
        println!("[{}]: {}", message.display_name, message.content);
    }));

    loop {
        print!("[You]: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input == "/dump" {
            println!("--- Conversation Dump ---");
            for message in conversation.messages() {
                let name = match message.message_type {
                    MessageType::User => "You",
                    MessageType::Assistant => message.display_name.as_str(),
                    MessageType::Tool => "Tool",
                };
                println!("> [{}]: {}", name, message.content);
            }
            println!("--- End of Dump ---\n");
            continue;
        }

        conversation.add_user_message(input.to_string());
        match application.infer(&ctx, &mut conversation).await {
            Ok(_) => {}
            Err(e) => {
                println!("Error during inference: {}", e.internal_message);
            }
        }
    }

    Ok(())
}
