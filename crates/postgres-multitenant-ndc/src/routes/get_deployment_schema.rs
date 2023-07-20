use axum::Json;

use ndc_client::models::SchemaResponse;

use crate::extract::Configuration;

#[axum_macros::debug_handler(state = crate::state::ServerState)]
pub async fn get_deployment_schema(
    Configuration(configuration): Configuration,
) -> Json<SchemaResponse> {
    // the deployment context extractor will error out if unable to find the context
    // this should not be fallible, if the context is there we can extrapolate the schema from it

    // TODO: figure out how to remove this clone.
    Json(configuration.schema.clone())
}
