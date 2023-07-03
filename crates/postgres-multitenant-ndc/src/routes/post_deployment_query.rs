#[allow(unused_imports)] // Server state is used by a dev time macro
use crate::{
    error::ServerError,
    extract::{Configuration, Pool},
    sql::query_builder::{BoundParam, QueryBuilder},
    state::ServerState,
};
use axum::Json;

use gdc_client::models::{QueryRequest, QueryResponse};
use sqlx::{types, Row};

#[axum_macros::debug_handler(state = ServerState)]
pub async fn post_deployment_query(
    Configuration(configuration): Configuration,
    Pool(pool): Pool,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ServerError> {
    println!("handling request");
    let (statement, parameters) = QueryBuilder::build_sql(&request, &configuration)?;

    println!("sql generated, executing...");
    let statement_string = statement.to_string();
    println!("{}", &statement_string);
    let query = sqlx::query(&statement_string);
    let query = parameters.into_iter().fold(query, |query, bound_param| {
        match bound_param {
            BoundParam::Number(number) => query.bind(number as i32),
            BoundParam::Value(value) => match value {
                serde_json::Value::Number(number) => query.bind(number.as_f64()),
                serde_json::Value::String(string) => query.bind(string),
                serde_json::Value::Bool(boolean) => query.bind(boolean),
                // feels like a hack.
                serde_json::Value::Null => query.bind(None::<bool>),
                serde_json::Value::Array(array) => query.bind(types::Json(array)),
                serde_json::Value::Object(object) => query.bind(types::Json(object)),
            },
        }
    });

    let result = query.fetch_one(&pool).await?;

    println!("sql response handled...");
    let value: types::JsonValue = result
        .try_get(0)
        .map_err(|err| ServerError::Internal(err.to_string()))?;
    // this parsing is technically not necessary, but useful for validation during development, and to ensure correctness.
    // todo: remove this, and instead send the first column of the first row as response without parsing or allocating additonal memory
    let response: QueryResponse =
        serde_json::from_value(value).map_err(|err| ServerError::Internal(err.to_string()))?;

    Ok(Json(response))
}
