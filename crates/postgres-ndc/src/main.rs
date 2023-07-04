#[macro_use]
extern crate log;

use postgres_ndc::routes;
use std::env;

#[tokio::main]
async fn main() {
    // allow server port to be set via PORT env var
    // this may not be the place for this, but yolo for now
    let port = env::var("PORT").unwrap_or("3000".to_string());
    let address = format!("0.0.0.0:{}", port);

    env_logger::init();
    info!("starting postgres-ndc client");

    match routes::router().await {
        Err(err) => log::error!("{}", err.to_string()),
        Ok(app) => {
            let server =
                axum::Server::bind(&address.parse().unwrap()).serve(app.into_make_service());

            log::info!("Starting axum server at {}", address);

            server.await.unwrap();
        }
    }
}
