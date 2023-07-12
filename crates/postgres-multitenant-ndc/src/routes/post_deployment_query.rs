use axum::Json;

use gdc_client::models;
use query_engine::phases;

use crate::{
    error::ServerError,
    extract::{Configuration, Pool},
};

#[axum_macros::debug_handler(state = crate::state::ServerState)]
pub async fn post_deployment_query(
    // this will contain table of which tables live where, etc
    Configuration(configuration): Configuration,
    Pool(pool): Pool,
    Json(query_request): Json<models::QueryRequest>,
) -> Result<Json<models::QueryResponse>, ServerError> {
    tracing::info!("{}", serde_json::to_string(&query_request).unwrap());
    tracing::info!("{:?}", query_request);

    let plan = phases::translation::translate(&configuration.tables, query_request)?;

    let result = phases::execution::execute(pool, plan).await?;

    Ok(Json(result))
}
