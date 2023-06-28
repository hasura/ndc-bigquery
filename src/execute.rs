use serde_json::Value;
use std::collections::HashMap;

use crate::types::{output, translation};

pub fn execute(plan: translation::ExecutionPlan) -> output::QueryResponse {
    output::QueryResponse(vec![output::RowSet {
        rows: Some(vec![HashMap::from([(
            "x".to_string(),
            output::RowFieldValue::Column {
                value: Value::String("hi".to_string()),
            },
        )])]),
    }])
}
