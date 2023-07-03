use axum::http::StatusCode;

#[axum_macros::debug_handler()]
pub async fn get_health() -> StatusCode {
    StatusCode::NO_CONTENT
}
