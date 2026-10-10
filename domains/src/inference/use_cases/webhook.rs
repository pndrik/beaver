// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Value, json};

use crate::{
    core::models::{AppContext, AppError},
    inference::{
        Inference,
        models::{MessageType, Webhook},
    },
    tools::Tools,
};

impl Inference {
    pub async fn webhook_run(
        &self,
        ctx: &AppContext,
        hook: &Webhook,
        body: &Value,
        query: &Value,
        tools: &Tools,
    ) -> Result<(), AppError> {
        let template_values = json!({ "Body": body, "Query": query, "Ctx": ctx });
        let prompt = self
            .template_engine
            .render(ctx, &hook.handler.prompt, &template_values)
            .await?;

        let mut conversation = self
            .new_conversation(ctx, &hook.handler.agent, tools)
            .await?;

        conversation.add_user_message(prompt);
        self.infer(ctx, &mut conversation, tools).await?;

        for message in conversation.messages() {
            if message.message_type == MessageType::Assistant {
                ctx.logger
                    .trace(
                        ctx,
                        &format!("[{}]: {}", message.agent_name, message.content),
                    )
                    .await;
            }
        }

        Ok(())
    }
}
