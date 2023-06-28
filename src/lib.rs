use serde_json::Value;
use std::collections::HashMap;

pub mod input {

    /// This is the request body of the query POST endpoint
    pub struct QueryRequest {
        /// The name of a root field
        pub root_field: String,
        /// The query syntax tree
        pub query: Query,
    }

    pub enum Query {
        Raw(String),
    }
}

pub mod output {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use std::collections::HashMap;

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
}

pub mod translation {
    ////////////////

    pub struct ExecutionPlan {
        pub root_field: String,
        pub pre: Vec<DDL>,
        pub query: Query,
        pub post: Vec<DDL>,
    }
    pub struct SQL {
        pub sql: String,
        pub params: Vec<SQLValue>,
    }
    pub struct DDL(pub SQL);
    pub struct Query(pub SQL);
    pub enum SQLValue {
        Int4(i32),
    }
}

pub fn translate(query_request: input::QueryRequest) -> translation::ExecutionPlan {
    translation::ExecutionPlan {
        root_field: query_request.root_field,
        pre: vec![],
        query: translation::Query(translation::SQL {
            sql: "select * from t".to_string(),
            params: vec![],
        }),
        post: vec![],
    }
}

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
