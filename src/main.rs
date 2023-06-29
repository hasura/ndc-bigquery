use ndc_postgres::*;
use std::env;
use axum::{
    body::{Bytes, Full},
    response::Json,
    response::Response,
    routing::get,
    routing::post,
    Extension, Router,
};

use axum::extract::{Path, Query};
use serde_json::Value;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // allow server port to be set via PORT env var
    // this may not be the place for this, but yolo for now
    let port = env::var("PORT").unwrap_or("3000".to_string());
    let address = format!("0.0.0.0:{}", port);

    match connector::Connector::new().await {
        Err(err) => println!("{}", err),
        Ok(connector::Connector { pg_pool }) => {
            let app = Router::new()
                .route("/", get(root))
                .route("/id/:id", get(id))
                .route("/json", post(json))
                .route("/select", get(routes::query::query))
                .route("/query", post(json)) // just echo stuff back for now
                .layer(Extension(pg_pool));

            let server =
                axum::Server::bind(&address.parse().unwrap()).serve(app.into_make_service());

            println!("Starting axum server at {}", address);

            server.await.unwrap();
        }
    }
}

// dummy stuff. Will be removed later.

async fn root() -> &'static str {
    "hi"
}

async fn id(
    Path(user_id): Path<i64>,
    Query(params): Query<HashMap<String, String>>,
) -> Response<Full<Bytes>> {
    Response::builder()
        .header("x-powered-by", "benchmark")
        .header("Content-Type", "text/plain")
        .body(Full::from(format!("{} {}", user_id, params["name"])))
        .unwrap()
}

async fn json(Json(payload): Json<serde_json::Value>) -> Json<Value> {
    Json(payload)
}
