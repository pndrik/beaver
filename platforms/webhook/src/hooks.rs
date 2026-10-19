// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::extensions::AnyBody;
use crate::models::Response;
use app::App;
use app_domains::core::models::{AppContext, ErrorKind};

pub(crate) async fn handler(
    Extension(ctx): Extension<AppContext>,
    State(application): State<Arc<App>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    Path(name): Path<String>,
    AnyBody(body): AnyBody,
) -> (StatusCode, Json<Response>) {
    let token_bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let token_query = query.get("token").map(|s| s.as_str());

    let token = token_bearer.or(token_query).unwrap_or_default();
    if token.is_empty() {
        return unauthorized("Missing token");
    }

    let query: serde_json::Map<String, Value> = query
        .iter()
        .filter(|(key, _)| key.as_str() != "token")
        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
        .collect();
    let query = Value::Object(query);

    let hook = match application
        .domains
        .flows
        .resolve_hook(&ctx, &name, token, &body, &query)
        .await
    {
        Ok(hook) => hook,
        Err(err) => {
            return match err.kind {
                ErrorKind::Unauthorized => unauthorized("Invalid token"),
                ErrorKind::Validation => {
                    failure(StatusCode::UNPROCESSABLE_ENTITY, "Invalid arguments")
                }
                _ => failure(StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
            };
        }
    };

    let app = Arc::clone(&application);
    tokio::spawn(async move {
        if let Err(err) = app
            .domains
            .flows
            .run(
                &ctx,
                &hook.flow,
                &hook.arguments,
                &app.domains.inference,
                &app.domains.tools,
            )
            .await
        {
            ctx.logger
                .error(
                    &ctx,
                    &format!("Hook '{}' failed: {}", name, err.internal_message),
                )
                .await;
        }
    });

    (
        StatusCode::OK,
        Json(Response {
            success: true,
            message: "Inference started".to_string(),
        }),
    )
}

fn unauthorized(message: &str) -> (StatusCode, Json<Response>) {
    failure(StatusCode::UNAUTHORIZED, message)
}

fn failure(status: StatusCode, message: &str) -> (StatusCode, Json<Response>) {
    (
        status,
        Json(Response {
            success: false,
            message: message.to_string(),
        }),
    )
}
