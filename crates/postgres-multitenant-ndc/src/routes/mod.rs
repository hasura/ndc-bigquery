mod get_capabilities;
mod get_deployment_capabilities;
mod get_deployment_health;
mod get_deployment_schema;
mod get_health;
mod post_deployment_mutation;
mod post_deployment_mutation_explain;
mod post_deployment_query;
mod post_deployment_query_explain;
use crate::state::ServerState;
use axum::{
    routing::{get, post},
    Router,
};
use axum_tracing_opentelemetry::{opentelemetry_tracing_layer, response_with_trace_layer};

pub use get_capabilities::get_capabilities;
pub use get_deployment_capabilities::get_deployment_capabilities;
pub use get_deployment_health::get_deployment_health;
pub use get_deployment_schema::get_deployment_schema;
pub use get_health::get_health;
pub use post_deployment_mutation::post_deployment_mutation;
pub use post_deployment_mutation_explain::post_deployment_mutation_explain;
pub use post_deployment_query::post_deployment_query;
pub use post_deployment_query_explain::post_deployment_query_explain;

pub fn create_router(state: ServerState) -> Router {
    Router::new()
        .route("/capabilities", get(get_capabilities))
        .route(
            "/deployment/:deployment_id/capabilities",
            get(get_deployment_capabilities),
        )
        .route(
            "/deployment/:deployment_id/health",
            get(get_deployment_health),
        )
        .route(
            "/deployment/:deployment_id/schema",
            get(get_deployment_schema),
        )
        .route(
            "/deployment/:deployment_id/query",
            post(post_deployment_query),
        )
        .route(
            "/deployment/:deployment_id/explain",
            post(post_deployment_query_explain),
        )
        .route(
            "/deployment/:deployment_id/mutation",
            post(post_deployment_mutation),
        )
        .route(
            "/deployment/:deployment_id/mutation/explain",
            post(post_deployment_mutation_explain),
        )
        .with_state(state)
        .layer(response_with_trace_layer())
        .layer(opentelemetry_tracing_layer())
        .route("/health", get(get_health)) // don't trace this to avoid noise
}
