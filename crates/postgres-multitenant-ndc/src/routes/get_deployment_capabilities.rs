use axum::Json;

use super::get_capabilities::get_capabilities;
use crate::extract::Configuration;
use gdc_client::models::CapabilitiesResponse;

#[axum_macros::debug_handler(state = crate::state::ServerState)]
pub async fn get_deployment_capabilities(
    Configuration(_configuration): Configuration,
) -> Json<CapabilitiesResponse> {
    get_capabilities().await
}
