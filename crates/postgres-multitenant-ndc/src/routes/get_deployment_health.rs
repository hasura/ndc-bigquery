use axum::http::StatusCode;

#[allow(unused_imports)] // Server state is used by a dev time macro
use crate::{extract::Configuration, state::ServerState};

#[axum_macros::debug_handler(state = ServerState)]
pub async fn get_deployment_health(Configuration(_configuration): Configuration) -> StatusCode {
    // the context extractor will error if the deployment can't be found.
    // todo: check if connection pool is healthy.
    StatusCode::NO_CONTENT
}
