use axum::Json;

use gdc_client::models;
use query_engine::phases;

use crate::{
    error::ServerError,
    extract::{Configuration, Pool},
};

// extremely basic version of explain where we just return the SQL we have created

#[axum_macros::debug_handler(state = crate::state::ServerState)]
pub async fn post_deployment_query_explain(
    Configuration(configuration): Configuration,
    Pool(_pool): Pool,
    Json(query_request): Json<models::QueryRequest>,
) -> Result<Json<models::ExplainResponse>, ServerError> {
    tracing::info!("{}", serde_json::to_string(&query_request).unwrap());
    tracing::info!("{:?}", query_request);

    let statement = phases::translation::translate(&configuration.tables, query_request)?;

    let statement_string = statement.query().sql;

    let response = models::ExplainResponse {
        lines: vec![],
        query: statement_string,
    };

    Ok(Json(response))
}
