// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    flows::{Flows, models::Definition},
};

fn unauthorized(ctx: &AppContext) -> AppError {
    app_error!(
        Unauthorized,
        "invalid_credentials",
        "Invalid token provided for hook.",
        ctx.clone()
    )
}

impl Flows {
    pub async fn resolve_hook(
        &self,
        ctx: &AppContext,
        name: &str,
        token: &str,
    ) -> Result<Definition, AppError> {
        if token.is_empty() {
            ctx.logger
                .warn(ctx, &format!("Empty token for hook '{}'", name))
                .await;
            return Err(unauthorized(ctx));
        }

        match self.flow_provider.get_by_webhook(ctx, name, token).await {
            Ok(Some(flow)) => Ok(flow),
            Ok(None) => {
                ctx.logger
                    .warn(
                        ctx,
                        &format!("No flow found for hook '{}' or invalid token", name),
                    )
                    .await;
                Err(unauthorized(ctx))
            }
            Err(err) => {
                ctx.logger
                    .error(
                        ctx,
                        &format!(
                            "Failed to look up flows for hook '{}': {}",
                            name, err.internal_message
                        ),
                    )
                    .await;
                Err(err)
            }
        }
    }
}
