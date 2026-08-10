// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, StatusCode},
};
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
        return (
            StatusCode::UNAUTHORIZED,
            Json(Response {
                success: false,
                message: "Missing token".to_string(),
            }),
        );
    }

    let app = Arc::clone(&application);
    let ctx = ctx.clone();
    let name = name.clone();
    let body = body.clone();
    let token = token.to_string();
    tokio::spawn(async move {
        match app
            .domains
            .inference
            .webhook_call(&ctx, &name, &token, &body, &app.domains.skills)
            .await
        {
            Ok(_) => {}
            Err(err) => {
                ctx.logger
                    .error(
                        &ctx,
                        &format!("Webhook call failed: {}", err.internal_message),
                    )
                    .await;
            }
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
