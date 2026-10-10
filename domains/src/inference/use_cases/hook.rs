// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::Value;

use crate::{
    app_error,
    core::models::{AppContext, AppError, ErrorKind},
    inference::{
        Inference,
        models::{Flow, Webhook},
    },
    tools::Tools,
};

/// What a call to `/hooks/{name}` resolved to.
#[derive(Debug, Clone)]
pub enum HookTarget {
    Webhook(Webhook),
    Flow(Flow),
}

/// Compares in time that only depends on the lengths, not on where the inputs differ.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        diff |= (x ^ y) as usize;
    }
    diff == 0
}

fn unauthorized(ctx: &AppContext) -> AppError {
    app_error!(
        Unauthorized,
        "invalid_credentials",
        "Invalid token provided for hook.",
        ctx.clone()
    )
}

impl Inference {
    /// Finds the webhook or flow behind `name` and checks `token`. Every
    /// failure, including an unknown name, is `Unauthorized` so callers can't
    /// probe which hooks exist; the reason is logged instead.
    pub async fn resolve_hook(
        &self,
        ctx: &AppContext,
        name: &str,
        token: &str,
    ) -> Result<HookTarget, AppError> {
        let webhook = match self.webhook_provider.get(ctx, name).await {
            Ok(webhook) => Some(webhook),
            Err(err) if err.kind == ErrorKind::NotFound => None,
            Err(err) => {
                ctx.logger
                    .error(
                        ctx,
                        &format!(
                            "Failed to load webhook '{}': {}",
                            name, err.internal_message
                        ),
                    )
                    .await;
                return Err(unauthorized(ctx));
            }
        };

        let flow = match self.flow_provider.get_by_webhook(ctx, name).await {
            Ok(flow) => flow,
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
                None
            }
        };

        let (target, expected_token) = match (webhook, flow) {
            (Some(_), Some(flow)) => {
                ctx.logger
                    .error(
                        ctx,
                        &format!(
                            "Hook '{}' is defined by both a webhook and flow '{}'; running neither",
                            name, flow.metadata.name
                        ),
                    )
                    .await;
                return Err(unauthorized(ctx));
            }
            (Some(webhook), None) => {
                let token = webhook.metadata.token.clone();
                (HookTarget::Webhook(webhook), token)
            }
            (None, Some(flow)) => {
                let token = flow
                    .spec
                    .triggers
                    .webhook
                    .as_ref()
                    .map(|webhook| webhook.token.clone())
                    .unwrap_or_default();
                (HookTarget::Flow(flow), token)
            }
            (None, None) => {
                ctx.logger
                    .warn(
                        ctx,
                        &format!("No webhook or flow found for hook '{}'", name),
                    )
                    .await;
                return Err(unauthorized(ctx));
            }
        };

        if token.is_empty()
            || expected_token.is_empty()
            || !constant_time_eq(token.as_bytes(), expected_token.as_bytes())
        {
            ctx.logger
                .warn(ctx, &format!("Invalid token for hook '{}'", name))
                .await;
            return Err(unauthorized(ctx));
        }

        Ok(target)
    }

    pub async fn run_hook(
        &self,
        ctx: &AppContext,
        target: &HookTarget,
        body: &Value,
        query: &Value,
        tools: &Tools,
    ) -> Result<(), AppError> {
        match target {
            HookTarget::Webhook(webhook) => {
                self.webhook_run(ctx, webhook, body, query, tools).await
            }
            HookTarget::Flow(flow) => self.flow_run(ctx, flow, body, query, tools).await,
        }
    }
}
