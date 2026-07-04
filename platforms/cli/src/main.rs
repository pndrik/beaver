// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::io::{self, Write};

use app_domains::inference::models::MessageType;

#[tokio::main]
async fn main() {
    let ctx = app::context("local".to_string())
        .await
        .expect("Failed to create application context");
    let application = app::bootstrap(&ctx)
        .await
        .expect("Failed to bootstrap application");
    let mut conversation = application
        .new_conversation(&ctx, "conductor")
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
}
