// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use axum::extract::State;
use axum::{
    Router,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::any,
};
use std::{collections::HashMap, io, sync::Arc, thread};
use tokio::runtime::Builder;

use app::App;
use app_domains::core::models::{AppContext, AppError};

mod extensions;
mod hooks;
mod models;

const MIN_WORKER_THREADS: usize = 6;
fn determine_thread_count() -> (usize, Option<io::Error>) {
    match thread::available_parallelism() {
        Ok(threads) => (threads.get().max(MIN_WORKER_THREADS), None),
        Err(e) => (MIN_WORKER_THREADS, Some(e)),
    }
}

async fn get_bind_addr(ctx: &AppContext) -> Result<String, AppError> {
    let port = ctx.configuration().get_int(ctx, "webhooks.port").await?;
    let bind = ctx.configuration().get_string(ctx, "webhooks.bind").await?;

    Ok(format!("{}:{}", bind, port))
}

/// Header names are lowercase; repeated headers are joined with ", ", non-UTF-8 values are skipped.
fn header_metadata(headers: &HeaderMap) -> HashMap<String, String> {
    let mut metadata: HashMap<String, String> = HashMap::new();
    for (name, value) in headers {
        let Ok(value) = value.to_str() else { continue };
        metadata
            .entry(name.as_str().to_string())
            .and_modify(|existing| {
                existing.push_str(", ");
                existing.push_str(value);
            })
            .or_insert_with(|| value.to_string());
    }
    metadata
}

async fn context_middleware(
    State(application): State<Arc<App>>,
    mut req: Request,
    next: Next,
) -> Response {
    let ctx = match application
        .get_context(None, header_metadata(req.headers()))
        .await
    {
        Ok(ctx) => ctx,
        Err(e) => {
            let boot = application.boot_context();
            boot.logger()
                .error(
                    boot,
                    &format!("Failed to create request context: {}", e.internal_message),
                )
                .await;

            let status = StatusCode::from_u16(e.kind.http_status())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            return hooks::failure(status, status.canonical_reason().unwrap_or("Error"))
                .into_response();
        }
    };

    ctx.logger()
        .info(&ctx, &format!("[{}] {}", req.method(), req.uri().path()))
        .await;

    req.extensions_mut().insert(ctx);
    next.run(req).await
}

fn main() {
    let (worker_threads, parallelism_error) = determine_thread_count();
    let runtime = Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .thread_name("beaver-worker")
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    runtime.block_on(start(worker_threads, parallelism_error));
}

async fn start(worker_threads: usize, parallelism_error: Option<io::Error>) {
    let application = Arc::new(
        app::bootstrap()
            .await
            .expect("Failed to initialize application"),
    );

    let ctx = application
        .get_context(Some("boot-webhook".to_string()), HashMap::new())
        .await
        .expect("Failed to get application context");

    if let Some(e) = parallelism_error {
        ctx.logger()
            .warn(
                &ctx,
                &format!("Failed to determine available parallelism: {}", e),
            )
            .await;
    }
    ctx.logger()
        .info(
            &ctx,
            &format!("Starting server with {} worker threads", worker_threads),
        )
        .await;

    let bind_addr = get_bind_addr(&ctx)
        .await
        .expect("Failed to get bind address");

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .expect("Failed to bind to address");

    let app_router = Router::new()
        .route("/hooks/{name}", any(hooks::handler))
        .with_state(application.clone())
        .layer(middleware::from_fn_with_state(
            application.clone(),
            context_middleware,
        ));

    let router = Router::new()
        .route("/healthz", any(|| async { "OK" }))
        .merge(app_router);

    axum::serve(listener, router).await.unwrap();
}
