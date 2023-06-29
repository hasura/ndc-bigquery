/// An api call to `/query` should end up here.
use sqlx;

use axum::response::Json;
use axum::Extension;

use crate::*;
use serde_json::Value;
use std::collections::HashMap;

pub async fn query(axum::Extension(pool): Extension<sqlx::PgPool>) -> Json<Value> {
    let plan = phases::translation::translate(empty_query_request());

    match phases::execution::execute(pool, plan).await {
        Err(err) => Json(Value::String(err.to_string())),
        Ok(types::output::QueryResponse(results)) => match serde_json::to_value(results) {
            Err(err) => Json(Value::String(err.to_string())),
            Ok(value) => Json(value),
        },
    }
}
// utils

fn empty_query_request() -> types::input::QueryRequest {
    types::input::QueryRequest {
        table: "bamba".to_string(),
        query: empty_query(),
        arguments: HashMap::new(),
        table_relationships: HashMap::new(),
        variables: None,
    }
}

fn empty_query() -> types::input::Query {
    types::input::Query {
        aggregates: None,
        fields: None,
        limit: None,
        offset: None,
        order_by: None,
        predicate: None,
    }
}
