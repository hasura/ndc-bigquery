/// gdc_client::models contains the interface with which we communicate with v3.
/// However, that interface specifies the results we return from the database as rust values,
/// and we'd like the database to convert all of the data to json and for us to just
/// send to it v3 without doing any serializing and deserializing.
///
/// We suspect that the models API will change to avoid serialization, so we implement our
/// query builder with that assumption and then parse the data back from the database and
/// serialize it to what v3 expects.
///
/// Essentially - we are building against the database how we expect it to work,
/// and then hack the results from the database to generate a result that matches what v3
///
/// *currently* expects. We expect to remove this module in its entirely soon.
use gdc_client::models;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_with::skip_serializing_none;
use std::collections::HashMap;

pub fn rows_to_response(sets_of_rows: Vec<serde_json::Value>) -> models::QueryResponse {
    let rowsets = sets_of_rows
        .into_iter()
        .map(|rows| RowSet {
            aggregates: None,
            rows: serde_json::from_value(rows).unwrap(),
        })
        .collect();

    response_to_response(QueryResponse(rowsets))
}

pub fn response_to_response(QueryResponse(response): QueryResponse) -> models::QueryResponse {
    models::QueryResponse(response.into_iter().map(rowset_to_rowset).collect())
}

pub fn rowset_to_rowset(rowset: RowSet) -> models::RowSet {
    models::RowSet {
        aggregates: rowset.aggregates,
        rows: rowset.rows.map(|vec| {
            vec.into_iter()
                .map(|obj| {
                    obj.into_iter()
                        .map(|(name, value)| {
                            (
                                name,
                                match value {
                                    RowFieldValue::Relationship { rows } => {
                                        models::RowFieldValue::Relationship {
                                            rows: rowset_to_rowset(rows),
                                        }
                                    }
                                    RowFieldValue::Column(json) => {
                                        models::RowFieldValue::Column { value: json }
                                    }
                                },
                            )
                        })
                        .collect()
                })
                .collect()
        }),
    }
}

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
    Column(serde_json::Value),
}
