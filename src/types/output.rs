/// Output types for the connector. Also copy-pasted.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

// ANCHOR: QueryResponse
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
/// Query responses may return multiple RowSets when using foreach queries
/// Else, there should always be exactly one RowSet
pub struct QueryResponse(pub Vec<RowSet>);
// ANCHOR_END: QueryResponse

// ANCHOR: RowSet
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RowSet {
    /// The results of the aggregates returned by the query
    pub aggregates: Option<HashMap<String, serde_json::Value>>,
    /// The rows returned by the query, corresponding to the query's fields
    pub rows: Option<Vec<HashMap<String, RowFieldValue>>>,
}
// ANCHOR_END: RowSet

// ANCHOR: RowFieldValue
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RowFieldValue {
    Relationship { rows: RowSet },
    Column { value: serde_json::Value },
}
// ANCHOR_END: RowFieldValue
