use axum::Json;

use crate::{
    error::ServerError,
    extract::{Configuration, Pool},
};

use gdc_client::models::{MutationRequest, MutationResponse};

#[axum_macros::debug_handler(state = crate::state::ServerState)]
pub async fn post_deployment_mutation(
    Configuration(_configuration): Configuration,
    Pool(_pool): Pool,
    Json(_request): Json<MutationRequest>,
) -> Result<Json<MutationResponse>, ServerError> {
    todo!()
}
