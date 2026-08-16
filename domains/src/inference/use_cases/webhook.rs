// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Value, json};

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::{Inference, models::MessageType},
    tools::Tools,
};

impl Inference {
    pub async fn webhook_call(
        &self,
        ctx: &AppContext,
        name: &str,
        token: &str,
        body: &Value,
        tools: &Tools,
    ) -> Result<(), AppError> {
        let hook = self.webhook_provider.get(ctx, name).await?;
        if token != "" && hook.metadata.token != token {
            return Err(app_error!(
                Unauthorized,
                "invalid_credentials",
                "Invalid token provided for webhook.",
                ctx.clone()
            ));
        }

        let template_values = json!({ "Body": &body, "Ctx": ctx });
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
                        &format!("[{}]: {}", message.display_name, message.content),
                    )
                    .await;
            }
        }

        Ok(())
    }
}
