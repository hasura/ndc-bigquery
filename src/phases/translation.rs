/// Translate the incoming QueryRequest to an ExecutionPlan (SQL) to be run against the database.
/// Also exports the SQL AST types and the low-level string representation of a SQL query type.
pub mod convert;
pub mod sql_ast;
pub mod sql_string;

use crate::types::input;

#[derive(Debug)]
/// Definition of an execution plan to be run against the database.
pub struct ExecutionPlan {
    pub root_field: String,
    /// Run before the query. Should be a sql_ast in the future.
    pub pre: Vec<sql_string::DDL>,
    /// The query.
    pub query: sql_ast::Select,
    /// Run after the query. Should be a sql_ast in the future.
    pub post: Vec<sql_string::DDL>,
}

impl ExecutionPlan {
    /// Extract the query component as SQL.
    pub fn query(&self) -> sql_string::SQL {
        select_to_sql(&self.query)
    }
}

pub fn select_to_sql(select: &sql_ast::Select) -> sql_string::SQL {
    let mut sql = sql_string::SQL::new();
    select.to_sql(&mut sql);
    sql
}

/// Translate the incoming QueryRequest to an ExecutionPlan (SQL) to be run against the database.
pub fn translate(query_request: input::QueryRequest) -> ExecutionPlan {
    let select = sql_ast::simple_select(
        vec![
            (
                sql_ast::ColumnAlias {
                    unique_index: 0,
                    name: "City".to_string(),
                },
                sql_ast::Expression::ColumnName(sql_ast::ColumnName::TableColumn(
                    "City".to_string(),
                )),
            ),
            (
                sql_ast::ColumnAlias {
                    unique_index: 0,
                    name: "Country".to_string(),
                },
                sql_ast::Expression::ColumnName(sql_ast::ColumnName::TableColumn(
                    "Country".to_string(),
                )),
            ),
        ],
        sql_ast::From::Table {
            name: sql_ast::TableName::DBTable(query_request.table.clone()),
            alias: sql_ast::TableAlias {
                unique_index: 0,
                name: query_request.table.clone(),
            },
        },
    );
    simple_exec_plan(query_request.table, select)
}

/// A simple execution plan with only a root field and a query.
pub fn simple_exec_plan(root_field: String, query: sql_ast::Select) -> ExecutionPlan {
    ExecutionPlan {
        root_field,
        pre: vec![],
        query,
        post: vec![],
    }
}
