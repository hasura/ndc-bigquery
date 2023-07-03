/// An api call to `/query` should end up here.
use sqlx;

use axum::response::Json;
use axum::Extension;

use crate::*;
use serde_json::Value;

pub async fn query(
    axum::Extension(pool): Extension<sqlx::PgPool>,
    Json(query_request): Json<types::input::QueryRequest>,
) -> Json<Value> {
    println!("{}", serde_json::to_string(&query_request).unwrap());
    println!("{:?}", query_request);
    match phases::translation::translate(query_request) {
        Err(err) => Json(Value::String(err.to_string())),
        Ok(plan) => match phases::execution::execute(pool, plan).await {
            Err(err) => Json(Value::String(err.to_string())),
            Ok(types::output::QueryResponse(results)) => match serde_json::to_value(results) {
                Err(err) => Json(Value::String(err.to_string())),
                Ok(value) => Json(value),
            },
        },
    }
}
