// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use tracing::Instrument;

use app::App;
use app_domains::core::{fatal, models::AppContext};

mod conversation;

#[tokio::main]
async fn main() {
    let application = app::bootstrap()
        .await
        .unwrap_or_else(|e| fatal("Failed to bootstrap application", &e.internal_message));

    let ctx = application
        .get_context(Some("cli".to_string()))
        .await
        .unwrap_or_else(|e| fatal("Failed to get application context", &e.internal_message));

    run(&ctx, &application)
        .instrument(application.context_span(&ctx))
        .await;
}

async fn run(ctx: &AppContext, application: &App) {
    // TBD: Needs improvement
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "conversation" {
        let agent = args[2].clone();

        conversation::run_conversation(ctx, application, &agent)
            .await
            .expect("Failed to run conversation");
    } else if args.len() >= 3 && args[1] == "tools" && args[2] == "list" {
        for tool in application
            .domains
            .tools
            .list_all(ctx)
            .await
            .expect("Failed to list tools")
        {
            println!("{} - {}", tool.name, tool.description);
        }
    } else if args.len() >= 3 && args[1] == "agents" && args[2] == "list" {
        for agent in application
            .domains
            .inference
            .agents_list(ctx)
            .await
            .expect("Failed to list agents")
        {
            println!(
                "{} ({}) - {}",
                agent.metadata.name, agent.metadata.display_name, agent.metadata.description
            );
        }
    } else {
        println!("Usage: {} <command> [args]", args[0]);
        println!("Commands:");
        println!("  conversation <agent_name> - Start a conversation with the specified agent");
        println!("  tools list                - List available tools");
        println!("  agents list               - List available agents");
    }
}
