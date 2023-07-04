pub mod query;

use log;

use crate::*;
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

pub async fn router() -> Result<Router, sqlx::Error> {
    log::info!("Connecting to postgres...");
    let connector::Connector { pg_pool } = connector::Connector::new().await?;
    let app = Router::new()
        .route("/", get(root))
        .route("/id/:id", get(id))
        .route("/json", post(json))
        .route("/query", post(routes::query::query))
        .layer(Extension(pg_pool));
    Ok(app)
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
