use axum::http::StatusCode;

use crate::extract::Configuration;

#[axum_macros::debug_handler(state = crate::state::ServerState)]
pub async fn get_deployment_health(Configuration(_configuration): Configuration) -> StatusCode {
    // the context extractor will error if the deployment can't be found.
    // todo: check if connection pool is healthy.
    StatusCode::NO_CONTENT
}
