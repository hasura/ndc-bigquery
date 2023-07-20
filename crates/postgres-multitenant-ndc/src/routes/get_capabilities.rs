use axum::Json;
use ndc_client::models::{Capabilities, CapabilitiesResponse, QueryCapabilities};

#[axum_macros::debug_handler]
pub async fn get_capabilities() -> Json<CapabilitiesResponse> {
    let empty = serde_json::to_value(()).unwrap();
    Json(CapabilitiesResponse {
        versions: "^0.0.0".into(),
        capabilities: Capabilities {
            explain: Some(empty.clone()),
            query: Some(QueryCapabilities {
                foreach: Some(empty),
                order_by_aggregate: None,
                relation_comparisons: None,
            }),
            mutations: None,
            relationships: None,
        },
    })
}
