use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use query_engine::phases::{execution, translation};
use serde::Serialize;

pub enum ServerError {
    Internal(String),
    QueryError(String),
    DatabaseError(String),
    DeploymentNotFound,
    DeploymentIdMissingOrInvalid,
}

#[derive(Serialize)]
struct JsonErrorResponse {
    message: String,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ServerError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ServerError::QueryError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ServerError::DeploymentNotFound => (
                StatusCode::NOT_FOUND,
                "Deployment config not found".to_string(),
            ),

            ServerError::DeploymentIdMissingOrInvalid => (
                StatusCode::BAD_REQUEST,
                "Deployment id missing or invalid".to_string(),
            ),
            ServerError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        tracing::error!("Returning error: {message} with status code: {status}");
        (status, Json(JsonErrorResponse { message })).into_response()
    }
}

impl From<sqlx::Error> for ServerError {
    fn from(value: sqlx::Error) -> Self {
        ServerError::DatabaseError(value.to_string())
    }
}

impl From<String> for ServerError {
    fn from(value: String) -> Self {
        ServerError::Internal(value)
    }
}

impl From<translation::Error> for ServerError {
    fn from(value: translation::Error) -> Self {
        ServerError::QueryError(value.to_string())
    }
}

impl From<execution::Error> for ServerError {
    fn from(err: execution::Error) -> Self {
        match err {
            execution::Error::Query(err) => ServerError::QueryError(err),
            execution::Error::DB(err) => ServerError::DatabaseError(err.to_string()),
        }
    }
}
