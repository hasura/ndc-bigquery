/// Translate the incoming QueryRequest to an ExecutionPlan (SQL) to be run against the database.
/// Also exports the SQL AST types and the low-level string representation of a SQL query type.
pub mod convert;
pub mod sql_ast;
pub mod sql_string;

use gdc_client::models;
use log;

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
pub fn translate(query_request: models::QueryRequest) -> Result<ExecutionPlan, Error> {
    let mut translate = Translate::new();
    translate.translate(query_request)
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

/// State for the translation phase
pub struct Translate {
    /// give each alias a unique name using this number.
    pub unique_index: u64,
}

impl Default for Translate {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new translation context and start translating a query request to a sql ast.
impl Translate {
    /// Create a transation context.
    pub fn new() -> Translate {
        Translate { unique_index: 0 }
    }

    /// Translate a query request to sql ast.
    pub fn translate(
        &mut self,
        query_request: models::QueryRequest,
    ) -> Result<ExecutionPlan, Error> {
        // translate fields to select list
        let fields = match query_request.query.fields {
            None => Error::new("no fields in query request."),
            Some(fields) => Ok(fields),
        }?;

        // translate fields to columns or relationships.
        let columns: Vec<(sql_ast::ColumnAlias, sql_ast::Expression)> = fields
            .into_iter()
            .flat_map(|(alias, field)| match field {
                models::Field::Column { column, .. } => Ok(make_column(
                    query_request.table.clone(),
                    column,
                    self.make_column_alias(alias),
                )),
                models::Field::Relationship { .. } => Error::new("relationships are not supported"),
            })
            .collect::<Vec<_>>();

        // construct a simple select with the table name, alias, and selected columns.
        let mut select = sql_ast::simple_select(
            columns,
            sql_ast::From::Table {
                // @todo: how do we know the name of the table schema? assume public for now.
                name: sql_ast::TableName::from_public(query_request.table.clone()),
                alias: self.make_table_alias(query_request.table.clone()),
            },
        );

        // translate where
        select.where_ = sql_ast::Where(match query_request.query.predicate {
            None => sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
            Some(predicate) => self.translate_expression(&query_request.table, predicate),
        });

        // translate limit and offset
        select.limit = sql_ast::Limit {
            limit: query_request.query.limit,
            offset: query_request.query.offset,
        };

        // log and return
        log::info!("SQL AST: {:?}", select);
        Ok(simple_exec_plan(query_request.table, select))
    }

    /// create column aliases using this function so they get a unique index.
    fn make_column_alias(&mut self, name: String) -> sql_ast::ColumnAlias {
        let index = self.unique_index;
        self.unique_index += 1;
        sql_ast::ColumnAlias {
            unique_index: index,
            name,
        }
    }
    /// create table aliases using this function so they get a unique index.
    fn make_table_alias(&mut self, name: String) -> sql_ast::TableAlias {
        let index = self.unique_index;
        self.unique_index += 1;
        sql_ast::TableAlias {
            unique_index: index,
            name,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn translate_expression(
        &mut self,
        table: &String,
        predicate: models::Expression,
    ) -> sql_ast::Expression {
        match predicate {
            models::Expression::And { expressions } => expressions
                .into_iter()
                .map(|expr| self.translate_expression(table, expr))
                .fold(
                    sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
                    |acc, expr| sql_ast::Expression::And {
                        left: Box::new(acc),
                        right: Box::new(expr),
                    },
                ),
            models::Expression::Or { expressions } => expressions
                .into_iter()
                .map(|expr| self.translate_expression(table, expr))
                .fold(
                    sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
                    |acc, expr| sql_ast::Expression::Or {
                        left: Box::new(acc),
                        right: Box::new(expr),
                    },
                ),
            models::Expression::Not { expression } => {
                sql_ast::Expression::Not(Box::new(self.translate_expression(table, *expression)))
            }
            models::Expression::BinaryComparisonOperator {
                column,
                operator,
                value,
            } => sql_ast::Expression::BinaryOperator {
                left: Box::new(translate_comparison_target(table, *column)),
                operator: match *operator {
                    models::BinaryComparisonOperator::Equal => sql_ast::BinaryOperator::Equals,
                    _ => sql_ast::BinaryOperator::Equals,
                },
                right: Box::new(translate_comparison_value(table, *value)),
            },
            // dummy
            _ => sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
        }
    }
}
/// translate a comparison target.
fn translate_comparison_target(
    table: &str,
    column: models::ComparisonTarget,
) -> sql_ast::Expression {
    match column {
        models::ComparisonTarget::Column { name, .. } => {
            sql_ast::Expression::ColumnName(sql_ast::ColumnName::TableColumn {
                table: table.to_owned(),
                name,
            })
        }
        // dummy
        _ => sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
    }
}

/// translate a comparison value.
fn translate_comparison_value(table: &str, value: models::ComparisonValue) -> sql_ast::Expression {
    match value {
        models::ComparisonValue::Column { column } => translate_comparison_target(table, *column),
        models::ComparisonValue::Scalar { value } => match value {
            serde_json::Value::Number(num) => sql_ast::Expression::Value(sql_ast::Value::Int4(
                num.as_i64().unwrap().try_into().unwrap(),
            )),
            serde_json::Value::Bool(b) => sql_ast::Expression::Value(sql_ast::Value::Bool(b)),
            // dummy
            _ => sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
        },
        // dummy
        _ => sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
    }
}

/// generate a column expression.
fn make_column(
    table: String,
    name: String,
    alias: sql_ast::ColumnAlias,
) -> (sql_ast::ColumnAlias, sql_ast::Expression) {
    (
        alias,
        sql_ast::Expression::ColumnName(sql_ast::ColumnName::TableColumn { table, name }),
    )
}

/// A type for translation errors.
pub struct Error(pub String);

impl Error {
    pub fn new<T>(error: &str) -> Result<T, Error> {
        Err(Error(format!("Translation failed: {}", error)))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Error(err) = self;
        write!(f, "{}", err)
    }
}
