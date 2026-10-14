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
use app_domains::core::models::AppContext;

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

    let flow = match application
        .domains
        .flows
        .resolve_hook(&ctx, &name, token)
        .await
    {
        Ok(flow) => flow,
        Err(_) => return unauthorized("Invalid token"),
    };

    let query: serde_json::Map<String, Value> = query
        .into_iter()
        .filter(|(key, _)| key != "token")
        .map(|(key, value)| (key, Value::String(value)))
        .collect();
    let query = Value::Object(query);

    let app = Arc::clone(&application);
    tokio::spawn(async move {
        if let Err(err) = app
            .domains
            .flows
            .run(
                &ctx,
                &flow,
                &body,
                &query,
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
    (
        StatusCode::UNAUTHORIZED,
        Json(Response {
            success: false,
            message: message.to_string(),
        }),
    )
}
