// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::Value;

use crate::{
    app_error,
    core::models::{AppContext, AppError},
    flows::{Flows, models::Hook},
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
    /// Checks `token` for the hook `name` and maps the payload to the flow's
    /// arguments. Fails with `Unauthorized` for an unknown hook or a wrong
    /// token and with `Validation` for invalid arguments.
    pub async fn resolve_hook(
        &self,
        ctx: &AppContext,
        name: &str,
        token: &str,
        body: &Value,
        query: &Value,
    ) -> Result<Hook, AppError> {
        if token.is_empty() {
            ctx.logger
                .warn(ctx, &format!("Empty token for hook '{}'", name))
                .await;
            return Err(unauthorized(ctx));
        }

        let flow = match self.flow_provider.get_by_webhook(ctx, name, token).await {
            Ok(Some(flow)) => flow,
            Ok(None) => {
                ctx.logger
                    .warn(
                        ctx,
                        &format!("No flow found for hook '{}' or invalid token", name),
                    )
                    .await;
                return Err(unauthorized(ctx));
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
                return Err(err);
            }
        };

        let arguments = flow
            .webhook_arguments(ctx, self.template_engine.as_ref(), body, query)
            .await;
        match arguments {
            Ok(arguments) => Ok(Hook { flow, arguments }),
            Err(err) => {
                ctx.logger
                    .warn(
                        ctx,
                        &format!("Hook '{}' rejected: {}", name, err.internal_message),
                    )
                    .await;
                Err(err)
            }
        }
    }
}
