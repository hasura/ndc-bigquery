mod error;
mod extract;
mod state;
mod routes;
mod sync;

extern crate log;

use crate::sync::start_deployment_sync_thread;
use std::env;
use clap::Parser;
use state::ServerState;
use std::{ error::Error };
use routes::{create_router};

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

    env_logger::init();
    log::info!("Starting server on {}", address);

    let deployments_dir = server_options.deployments_dir;

    start_deployment_sync_thread(deployments_dir, state);

    axum::Server::bind(&address.parse()?)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
