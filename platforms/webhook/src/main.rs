// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use axum::extract::State;
use axum::{
    Router,
    extract::Request,
    middleware::{self, Next},
    response::Response,
    routing::any,
};
use std::sync::Arc;

use app::App;
use app_domains::core::models::{AppContext, AppError};

mod extensions;
mod hooks;
mod models;

async fn get_bind_addr(ctx: &AppContext) -> Result<String, AppError> {
    let port = ctx.configuration.get_int(&ctx, "webhooks.port").await?;
    let bind = ctx.configuration.get_string(&ctx, "webhooks.bind").await?;

    Ok(format!("{}:{}", bind, port))
}

async fn context_middleware(
    State(application): State<Arc<App>>,
    mut req: Request,
    next: Next,
) -> Response {
    let id = uuid::Uuid::new_v4().to_string();
    let ctx = application
        .get_context(id)
        .await
        .expect("Failed to get application context");

    ctx.logger
        .info(&ctx, &format!("[{}] {}", req.method(), req.uri().path()))
        .await;

    req.extensions_mut().insert(ctx);
    next.run(req).await
}

#[tokio::main]
async fn main() {
    let application = Arc::new(
        app::bootstrap()
            .await
            .expect("Failed to initialize application"),
    );

    let ctx = application
        .get_context("boot-webhook".to_string())
        .await
        .expect("Failed to get application context");

    let bind_addr = get_bind_addr(&ctx)
        .await
        .expect("Failed to get bind address");

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .expect("Failed to bind to address");

    let router = Router::new()
        .route("/healthz", any(|| async { "OK" }))
        .route("/hooks/{name}", any(hooks::handler))
        .with_state(application.clone())
        .layer(middleware::from_fn_with_state(
            application.clone(),
            context_middleware,
        ));

    axum::serve(listener, router).await.unwrap();
}
