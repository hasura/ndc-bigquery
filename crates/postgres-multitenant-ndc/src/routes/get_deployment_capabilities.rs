use axum::Json;
use gdc_client::models::{
    Capabilities, CapabilitiesResponse, MutationCapabilities, QueryCapabilities,
};

#[allow(unused_imports)] // Server state is used by a dev time macro
use crate::{extract::Configuration, state::ServerState};

#[axum_macros::debug_handler(state = ServerState)]
pub async fn get_deployment_capabilities(
    Configuration(_configuration): Configuration,
) -> Json<CapabilitiesResponse> {
    let empty = serde_json::to_value(()).unwrap();
    Json(CapabilitiesResponse {
        versions: "^1.0.0".into(),
        capabilities: Capabilities {
            explain: Some(empty.clone()),
            query: Some(QueryCapabilities {
                foreach: Some(empty.clone()),
                order_by_aggregate: Some(empty.clone()),
                relation_comparisons: Some(empty.clone()),
            }),
            mutations: Some(MutationCapabilities {
                returning: Some(empty.clone()),
                nested_inserts: Some(empty.clone()),
            }),
            relationships: Some(empty),
        },
    })
}
