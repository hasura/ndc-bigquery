use crate::types::{input, translation};

pub fn translate(query_request: input::QueryRequest) -> translation::ExecutionPlan {
    let select = translation::simple_select(
        vec![(
            translation::ColumnAlias {
                unique_index: 0,
                name: "x".to_string(),
            },
            translation::Expression::ColumnName(translation::ColumnName::TableColumn(
                "x".to_string(),
            )),
        )],
        translation::From::Table {
            name: translation::TableName::DBTable(query_request.table.clone()),
            alias: translation::TableAlias {
                unique_index: 0,
                name: query_request.table.clone(),
            },
        },
    );
    simple_exec_plan(query_request.table, select)
}

pub fn simple_exec_plan(
    root_field: String,
    query: translation::Select,
) -> translation::ExecutionPlan {
    translation::ExecutionPlan {
        root_field,
        pre: vec![],
        query,
        post: vec![],
    }
}
