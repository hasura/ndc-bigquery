/// An api call to `/query` should end up here.
use sqlx;

use axum::response::Json;
use axum::Extension;
use log;

use crate::*;
use gdc_client::models;
use serde_json::Value;

pub async fn query(
    axum::Extension(pool): Extension<sqlx::PgPool>,
    Json(query_request): Json<models::QueryRequest>,
) -> Json<Value> {
    log::info!(
        "Query request: {}",
        serde_json::to_string(&query_request).unwrap()
    );
    // println!("{:?}", query_request);
    log::info!("Translating query...");
    match phases::translation::translate(query_request) {
        Err(err) => {
            log::error!("Failed to translate query: {}", err.to_string());
            Json(Value::String(err.to_string()))
        }
        Ok(plan) => {
            log::info!("Executing query...");
            match phases::execution::execute(pool, plan).await {
                Err(err) => {
                    log::error!("Failed to execute query: {}", err.to_string());
                    Json(Value::String(err.to_string()))
                }
                Ok(models::QueryResponse(results)) => {
                    log::info!("Encoding results...");
                    match serde_json::to_value(results) {
                        Err(err) => {
                            log::error!("Failed to encode results: {}", err.to_string());
                            Json(Value::String(err.to_string()))
                        }
                        Ok(value) => {
                            log::info!("Sending results back to user...");
                            Json(value)
                        }
                    }
                }
            }
        }
    }
}
