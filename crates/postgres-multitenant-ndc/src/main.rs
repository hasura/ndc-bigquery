mod error;
mod extract;
mod routes;
mod state;
mod sync;

extern crate log;

use crate::sync::start_deployment_sync_thread;
use axum::BoxError;
use clap::Parser;
use routes::create_router;
use state::ServerState;
use std::env;

#[derive(Parser)]
struct ServerOptions {
    #[arg(long, value_name = "DEPLOYMENTS_DIR")]
    deployments_dir: String,
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    axum_tracing_opentelemetry::tracing_subscriber_ext::init_subscribers()?;

    let server_options = ServerOptions::parse();
    let state = ServerState::default();

    let router = create_router(state.clone());

    // allow server port to be set via PORT env var
    // this may not be the place for this, but yolo for now
    let port = env::var("PORT").unwrap_or("8081".to_string());
    let address = format!("0.0.0.0:{}", port);

    env_logger::init();
    tracing::info!("Starting server on {}", address);

    let deployments_dir = server_options.deployments_dir;

    start_deployment_sync_thread(deployments_dir, state);

    axum::Server::bind(&address.parse()?)
        .serve(router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// copied from https://github.com/davidB/axum-tracing-opentelemetry/blob/main/examples/otlp/src/main.rs
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::warn!("signal received, starting graceful shutdown");
    opentelemetry::global::shutdown_tracer_provider();
}
