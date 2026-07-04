// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Value, json};

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    inference::Inference,
    skills::Skills,
};

impl Inference {
    pub async fn webhook_call(
        &self,
        ctx: &AppContext,
        name: &str,
        token: &str,
        body: &Value,
        skills: &Skills,
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

        let template_values = json!({ "Body": &body });
        let prompt = self
            .template_engine
            .render(ctx, &hook.handler.prompt, &template_values)
            .await?;

        let mut conversation = self
            .new_conversation(ctx, &hook.handler.agent, skills)
            .await?;

        conversation.add_user_message(prompt);

        self.infer(ctx, &mut conversation, skills).await?;

        Ok(())
    }
}
