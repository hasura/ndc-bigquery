use axum::Json;
use sqlx::Row;

#[allow(unused_imports)] // Server state is used by a dev time macro
use crate::{
    error::ServerError,
    extract::{Configuration, Pool},
    sql::query_builder::{BoundParam, QueryBuilder},
    state::ServerState,
};
use gdc_client::models::{ExplainResponse, QueryRequest, QueryResponse};

#[axum_macros::debug_handler(state = ServerState)]
pub async fn post_deployment_query_explain(
    Configuration(configuration): Configuration,
    Pool(pool): Pool,
    Json(request): Json<QueryRequest>,
) -> Result<Json<ExplainResponse>, ServerError> {
    let (statement, parameters) = QueryBuilder::build_sql(&request, &configuration)?;

    let statement_string = statement.to_string();
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
                serde_json::Value::Array(array) => query.bind(sqlx::types::Json(array)),
                serde_json::Value::Object(object) => query.bind(sqlx::types::Json(object)),
            },
        }
    });

    let result = query.fetch_one(&pool).await?;

    let value: sqlx::types::JsonValue = result.get(0);

    let _response: QueryResponse =
        serde_json::from_value(value).map_err(|err| ServerError::Internal(err.to_string()))?;

    let response = ExplainResponse {
        lines: vec![],
        query: statement_string,
    };

    Ok(Json(response))
}
