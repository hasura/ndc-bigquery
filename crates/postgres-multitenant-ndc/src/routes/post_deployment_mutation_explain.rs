use axum::Json;

#[allow(unused_imports)] // Server state is used by a dev time macro
use crate::{
    error::ServerError,
    extract::{Configuration, Pool},
    state::ServerState,
};
use gdc_client::models::{ExplainResponse, MutationRequest};

#[axum_macros::debug_handler(state = ServerState)]
pub async fn post_deployment_mutation_explain(
    Configuration(_configuration): Configuration,
    Pool(_pool): Pool,
    Json(_request): Json<MutationRequest>,
) -> Result<Json<ExplainResponse>, ServerError> {
    todo!()
}
