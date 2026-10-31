// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use axum::{
    body::HttpBody,
    extract::{ConnectInfo, Request, State},
    http::{
        StatusCode,
        header::{CONTENT_LENGTH, USER_AGENT},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{net::SocketAddr, sync::Arc, time::Instant};
use tracing::{Instrument, error, info};

use crate::hooks;
use app::App;

pub(crate) async fn context_middleware(
    State(application): State<Arc<App>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    mut req: Request,
    next: Next,
) -> Response {
    let started = Instant::now();
    let ctx = match application.get_context(None).await {
        Ok(ctx) => ctx,
        Err(e) => {
            error!(error = %e.internal_message, "Failed to create request context");

            let status = StatusCode::from_u16(e.kind.http_status())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            return hooks::failure(status, status.canonical_reason().unwrap_or("Error"))
                .into_response();
        }
    };

    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let version = format!("{:?}", req.version());
    let user_agent = req
        .headers()
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();

    let span = application.context_span(&ctx);
    req.extensions_mut().insert(ctx.clone());
    let response = next.run(req).instrument(span.clone()).await;

    let latency_ms = (started.elapsed().as_secs_f64() * 100_000.0).round() / 100.0;
    let response_size = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok()?.parse::<u64>().ok())
        .or_else(|| response.body().size_hint().exact());
    span.in_scope(|| {
        info!(
            channel = "access log",
            method,
            path,
            version,
            status = response.status().as_u16(),
            latency_ms,
            client_ip = %client.ip(),
            user_agent,
            response_size,
        )
    });

    response
}
