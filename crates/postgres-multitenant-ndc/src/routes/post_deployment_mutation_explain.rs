use axum::Json;

use ndc_client::models::{ExplainResponse, MutationRequest};

use crate::{
    error::ServerError,
    extract::{Configuration, Pool},
};

#[axum_macros::debug_handler(state = crate::state::ServerState)]
pub async fn post_deployment_mutation_explain(
    Configuration(_configuration): Configuration,
    Pool(_pool): Pool,
    Json(_request): Json<MutationRequest>,
) -> Result<Json<ExplainResponse>, ServerError> {
    todo!()
}
