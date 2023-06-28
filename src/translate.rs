use crate::types::{input, translation};

pub fn translate(query_request: input::QueryRequest) -> translation::ExecutionPlan {
    translation::ExecutionPlan {
        root_field: query_request.table,
        pre: vec![],
        query: translation::Query(translation::SQL {
            sql: "select * from t".to_string(),
            params: vec![],
        }),
        post: vec![],
    }
}
