use axum::Json;

#[allow(unused_imports)] // Server state is used by a dev time macro
use crate::{
    error::ServerError,
    extract::{Configuration, Pool},
    state::ServerState,
};
use gdc_client::models;
use query_engine::{phases};

// extremely basic version of explain where we just return the SQL we have created

#[axum_macros::debug_handler(state = ServerState)]
pub async fn post_deployment_query_explain(
    Configuration(_configuration): Configuration,
    Pool(_pool): Pool,
    Json(query_request): Json<models::QueryRequest>,
) -> Result<Json<models::ExplainResponse>, ServerError> {

    log::info!("{}", serde_json::to_string(&query_request).unwrap());
    log::info!("{:?}", query_request);

    let statement = phases::translation::translate(query_request)?;

    let statement_string = statement.query().sql;

    let response = models::ExplainResponse {
        lines: vec![],
        query: statement_string,
    };

    Ok(Json(response))
}
