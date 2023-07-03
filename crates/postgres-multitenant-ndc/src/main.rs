mod error;
mod extract;
mod routes;
mod sql;
mod state;
use axum::{
    routing::{get, post},
    Router,
};
use std::env;
use clap::Parser;
use state::ServerState;
use std::{ error::Error, time::Duration};

use crate::state::update_deployments;

#[derive(Parser)]
struct ServerOptions {
    #[arg(long, value_name = "DEPLOYMENTS_DIR")]
    deployments_dir: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server_options = ServerOptions::parse();
    let state = ServerState::default();

    let router = create_router(state.clone());

    // allow server port to be set via PORT env var
    // this may not be the place for this, but yolo for now
    let port = env::var("PORT").unwrap_or("8081".to_string());
    let address = format!("0.0.0.0:{}", port);

    println!("Starting server on {}", address);

    let deployments_dir = server_options.deployments_dir;

    start_deployment_sync_thread(deployments_dir, state);

    axum::Server::bind(&address.parse()?)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

fn create_router(state: ServerState) -> Router {
    Router::new()
        .route("/health", get(routes::get_health))
        .route("/capabilities", get(routes::get_capabilities))
        .route(
            "/deployment/:deployment_id/capabilities",
            get(routes::get_deployment_capabilities),
        )
        .route(
            "/deployment/:deployment_id/health",
            get(routes::get_deployment_health),
        )
        .route(
            "/deployment/:deployment_id/schema",
            get(routes::get_deployment_schema),
        )
        .route(
            "/deployment/:deployment_id/query",
            post(routes::post_deployment_query),
        )
        .route(
            "/deployment/:deployment_id/query/explain",
            post(routes::post_deployment_query_explain),
        )
        .route(
            "/deployment/:deployment_id/mutation",
            post(routes::post_deployment_mutation),
        )
        .route(
            "/deployment/:deployment_id/mutation/explain",
            post(routes::post_deployment_mutation_explain),
        )
        .with_state(state)
}

pub fn start_deployment_sync_thread(base_dir: String, state: ServerState) {
    tokio::spawn(async move {
        println!("Started deployments sync thread");
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let base_dir = base_dir.clone();
            let state = state.clone();
            tokio::spawn(async move {
                if let Err(err) = update_deployments(base_dir, state).await {
                    println!("Error while updating deployments: {}", err)
                }
            });
        }
    });
}
