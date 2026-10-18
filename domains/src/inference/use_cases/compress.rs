// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{
        Inference,
        models::{Compression, Conversation, Message},
        use_cases::infer::find_inference_provider,
    },
    tools::models::ToolsConfiguration,
};

const PROMPT: &str = include_str!("compression.md");

fn transcript(messages: &[Message]) -> String {
    let mut lines = vec![];
    for message in messages {
        if let Some(content) = message.content() {
            lines.push(format!("[{}]: {}", message.name(), content));
        }
        if let Some(agent_message) = message.as_agent() {
            for call in &agent_message.tool_calls {
                lines.push(format!(
                    "[{}] called: {}({})",
                    message.name(),
                    call.name,
                    call.arguments_as_json()
                ));
            }
        }
    }

    lines.join("\n\n")
}

impl Inference {
    pub(crate) async fn compress(
        &self,
        ctx: &AppContext,
        conversation: &mut Conversation,
    ) -> Result<(), AppError> {
        if !conversation.needs_compression() {
            return Ok(());
        }

        let compression = conversation.agent().inference.compression.clone();
        let mut agent = conversation.agent().clone();
        if let Some(model) = compression.model {
            agent.inference.model = model;
        }
        agent.inference.compression = Compression::default();
        agent.tools = ToolsConfiguration::default();

        let context = conversation.context();
        let user_message = transcript(&context[..context.len() - 1]);
        let system_prompt = Message::system(
            compression.prompt.unwrap_or_else(|| PROMPT.to_string()),
            &agent,
        );
        let mut summarizer =
            Conversation::new(ctx, system_prompt, agent, vec![], conversation.chroot())?;
        summarizer.insert_message(Message::user(user_message, summarizer.agent()), None);

        ctx.logger
            .trace(
                ctx,
                &format!(
                    "Compressing conversation of agent '{}' ({} tokens, {} messages) with '{}'",
                    conversation.agent().metadata.name,
                    conversation.tokens(),
                    context.len(),
                    summarizer.agent().model().id()
                ),
            )
            .await;

        let failed = |message: &str| {
            app_error!(
                Internal,
                "inference_failed",
                &format!("Failed to compress the conversation: {}", message),
                ctx.clone()
            )
        };

        let provider =
            find_inference_provider(ctx, &self.inference_providers, summarizer.agent().model())
                .await
                .map_err(|e| failed(&e.internal_message))?;
        let options = summarizer.agent().inference.options.clone();
        provider
            .infer(ctx, &options, &mut summarizer)
            .await
            .map_err(|e| failed(&e.internal_message))?;

        let summary = summarizer
            .last()
            .and_then(|message| message.as_agent())
            .and_then(|message| message.content.clone())
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| failed("The summarizer returned no text"))?;
        conversation.insert_message(Message::summary(summary, conversation.agent()), Some(-2));

        Ok(())
    }
}
