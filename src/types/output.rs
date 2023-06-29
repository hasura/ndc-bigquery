/// Output types for the connector. Also copy-pasted.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// https://github.com/hasura/v3-experiments/blob/gdc-spec/crates/gdc-client/src/models.rs#L516
/// Query responses may return multiple RowSets when using foreach queries
/// Else, there should always be exactly one RowSet
pub struct QueryResponse(pub Vec<RowSet>);

#[derive(Serialize, Deserialize)]
pub struct RowSet {
    /// The rows returned by the query, corresponding to the query's fields
    pub rows: Option<Vec<HashMap<String, RowFieldValue>>>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum RowFieldValue {
    Column { value: Value },
}
