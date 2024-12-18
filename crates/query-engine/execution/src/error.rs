/// Errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Query(QueryError),
    #[error("{0}")]
    DB(gcp_bigquery_client::error::BQError),
}

/// Query planning error.
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Variable {0:?} not found.")]
    VariableNotFound(String),
    #[error("{0} are not supported.")]
    NotSupported(String),
    #[error("{0}")]
    DBError(gcp_bigquery_client::error::BQError),
    #[error("{0}")]
    DBConstraintError(gcp_bigquery_client::error::BQError),
    #[error("Mutation constraint failed.")]
    MutationConstraintFailed,
}
