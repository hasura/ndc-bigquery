/// Errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Query(QueryError),
}

/// Query planning error.
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Variable {0:?} not found.")]
    VariableNotFound(String),
    #[error("{0} are not supported.")]
    NotSupported(String),
}
