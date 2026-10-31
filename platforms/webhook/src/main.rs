// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use axum::{Router, middleware, routing::any};
use std::{io, net::SocketAddr, sync::Arc, thread};
use tokio::{net::TcpListener, runtime::Builder};
use tracing::{Instrument, info, warn};

use app_domains::core::{
    fatal,
    models::{AppContext, AppError},
};

mod extensions;
mod hooks;
mod middlewares;
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

fn main() {
    let (worker_threads, parallelism_error) = determine_thread_count();
    let runtime = Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .thread_name("beaver-worker")
        .enable_all()
        .build()
        .unwrap_or_else(|e| fatal("Failed to create Tokio runtime", &e));

    runtime.block_on(start(worker_threads, parallelism_error));
}

async fn listen(
    ctx: &AppContext,
    worker_threads: usize,
    parallelism_error: Option<io::Error>,
) -> TcpListener {
    if let Some(e) = parallelism_error {
        warn!(error = %e, "Failed to determine available parallelism");
    }
    info!(worker_threads, "Starting server");

    let bind_addr = get_bind_addr(ctx)
        .await
        .unwrap_or_else(|e| fatal("Failed to get bind address", &e.internal_message));
    let listener = TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| fatal("Failed to bind to address", &e));
    info!(listen_addr = %bind_addr, "Web server listening");

    listener
}

async fn start(worker_threads: usize, parallelism_error: Option<io::Error>) {
    let application = Arc::new(
        app::bootstrap()
            .await
            .unwrap_or_else(|e| fatal("Failed to bootstrap application", &e.internal_message)),
    );

    let ctx = application
        .get_context(Some("boot-webhook".to_string()))
        .await
        .unwrap_or_else(|e| fatal("Failed to get application context", &e.internal_message));

    let listener = listen(&ctx, worker_threads, parallelism_error)
        .instrument(application.context_span(&ctx))
        .await;

    let app_router = Router::new()
        .route("/hooks/{name}", any(hooks::handler))
        .with_state(application.clone())
        .layer(middleware::from_fn_with_state(
            application.clone(),
            middlewares::context_middleware,
        ));

    let router = Router::new()
        .route("/healthz", any(|| async { "OK" }))
        .merge(app_router);

    let service = router.into_make_service_with_connect_info::<SocketAddr>();
    if let Err(e) = axum::serve(listener, service).await {
        fatal("Web server failed", &e);
    }
}
