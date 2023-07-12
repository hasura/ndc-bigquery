/// Translate the incoming QueryRequest to an ExecutionPlan (SQL) to be run against the database.
/// Also exports the SQL AST types and the low-level string representation of a SQL query type.
pub mod convert;
pub mod sql_ast;
pub mod sql_string;
use crate::metadata;

use gdc_client::models;

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
pub fn translate(
    tables_info: &metadata::TablesInfo,
    query_request: models::QueryRequest,
) -> Result<ExecutionPlan, Error> {
    let mut translate = Translate::new();
    translate.translate(tables_info, query_request)
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
        metadata::TablesInfo(tables_info): &metadata::TablesInfo,
        query_request: models::QueryRequest,
    ) -> Result<ExecutionPlan, Error> {
        // find the table according to the metadata.
        let table: sql_ast::TableName = match tables_info.get(&query_request.table) {
            Some(t) => Ok(sql_ast::TableName::DBTable {
                schema: t.schema_name.clone(),
                table: t.table_name.clone(),
            }),
            None => Error::new("Table not found."),
        }?;
        let table_alias: sql_ast::TableAlias = self.make_table_alias(query_request.table.clone());
        let table_alias_name: sql_ast::TableName =
            sql_ast::TableName::AliasedTable(table_alias.clone());

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
                    table_alias_name.clone(),
                    column,
                    self.make_column_alias(alias),
                )),
                models::Field::Relationship { .. } => Error::new("relationships are not supported"),
            })
            .collect::<Vec<_>>();

        // construct a simple select with the table name, alias, and selected columns.
        let mut select = sql_ast::simple_select(columns);

        select.from = Some(sql_ast::From::Table {
            name: table,
            alias: table_alias.clone(),
        });

        // translate order_by
        select.order_by = self.translate_order_by(
            sql_ast::TableName::AliasedTable(table_alias),
            query_request.query.order_by,
        );

        // translate where
        select.where_ = sql_ast::Where(match query_request.query.predicate {
            None => sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
            Some(predicate) => self.translate_expression(&table_alias_name, predicate),
        });

        // translate limit and offset
        select.limit = sql_ast::Limit {
            limit: query_request.query.limit,
            offset: query_request.query.offset,
        };

        // wrap the sql in row_to_json and json_agg
        let final_select = sql_ast::select_as_json(
            select,
            self.make_column_alias("rows".to_string()),
            self.make_table_alias("root".to_string()),
        );

        // log and return
        tracing::info!("SQL AST: {:?}", final_select);
        Ok(simple_exec_plan(query_request.table, final_select))
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

    fn translate_order_by(
        &mut self,
        table: sql_ast::TableName,
        order_by: Option<models::OrderBy>,
    ) -> sql_ast::OrderBy {
        match order_by {
            None => sql_ast::OrderBy { elements: vec![] },
            Some(models::OrderBy { elements }) => {
                let order_by_parts = elements
                    .iter()
                    .map(|order_by| sql_ast::OrderByElement {
                        target: match &order_by.target {
                            models::OrderByTarget::Column { name, path } => {
                                if path.is_empty() {
                                    sql_ast::Expression::ColumnName(
                                        sql_ast::ColumnName::AliasedColumn {
                                            table: table.clone(),
                                            alias: self.make_column_alias(name.to_string()),
                                        },
                                    )
                                } else {
                                    panic!("relationships not implemented!")
                                }
                            }
                            models::OrderByTarget::SingleColumnAggregate { .. } => {
                                panic!("aggregates not implemented!")
                            }
                            models::OrderByTarget::StarCountAggregate { .. } => {
                                panic!("aggregates not implemented!")
                            }
                        },
                        direction: match order_by.order_direction {
                            models::OrderDirection::Asc => sql_ast::OrderByDirection::Asc,
                            models::OrderDirection::Desc => sql_ast::OrderByDirection::Desc,
                        },
                    })
                    .collect();
                sql_ast::OrderBy {
                    elements: order_by_parts,
                }
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn translate_expression(
        &mut self,
        table: &sql_ast::TableName,
        predicate: models::Expression,
    ) -> sql_ast::Expression {
        println!("{:?}", predicate);
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
                    models::BinaryComparisonOperator::Other { name } =>
                    // the strings we're matching against here (ie 'like') are best guesses for now, will
                    // need to update these as find out more
                    {
                        match &*name {
                            "like" => sql_ast::BinaryOperator::Like,
                            "nlike" => sql_ast::BinaryOperator::NotLike,
                            "ilike" => sql_ast::BinaryOperator::CaseInsensitiveLike,
                            "nilike" => sql_ast::BinaryOperator::NotCaseInsensitiveLike,
                            "similar" => sql_ast::BinaryOperator::Similar,
                            "nsimilar" => sql_ast::BinaryOperator::NotSimilar,
                            "regex" => sql_ast::BinaryOperator::Regex,
                            "nregex" => sql_ast::BinaryOperator::NotRegex,
                            "iregex" => sql_ast::BinaryOperator::CaseInsensitiveRegex,
                            "niregex" => sql_ast::BinaryOperator::NotCaseInsensitiveRegex,
                            "lt" => sql_ast::BinaryOperator::LessThan,
                            "lte" => sql_ast::BinaryOperator::LessThanOrEqualTo,
                            "gt" => sql_ast::BinaryOperator::GreaterThan,
                            "gte" => sql_ast::BinaryOperator::GreaterThanOrEqualTo,
                            _ => sql_ast::BinaryOperator::Equals,
                        }
                    }
                },
                right: Box::new(translate_comparison_value(table, *value)),
            },
            models::Expression::BinaryArrayComparisonOperator {
                column,
                operator,
                values,
            } => sql_ast::Expression::BinaryArrayOperator {
                left: Box::new(translate_comparison_target(table, *column)),
                operator: match *operator {
                    models::BinaryArrayComparisonOperator::In => sql_ast::BinaryArrayOperator::In,
                },
                right: values
                    .iter()
                    .map(|value| translate_comparison_value(table, value.clone()))
                    .collect(),
            },

            // dummy
            _ => sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
        }
    }
}
/// translate a comparison target.
fn translate_comparison_target(
    table: &sql_ast::TableName,
    column: models::ComparisonTarget,
) -> sql_ast::Expression {
    match column {
        models::ComparisonTarget::Column { name, .. } => {
            sql_ast::Expression::ColumnName(sql_ast::ColumnName::TableColumn {
                table: table.clone(),
                name,
            })
        }
        // dummy
        _ => sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
    }
}

/// translate a comparison value.
fn translate_comparison_value(
    table: &sql_ast::TableName,
    value: models::ComparisonValue,
) -> sql_ast::Expression {
    match value {
        models::ComparisonValue::Column { column } => translate_comparison_target(table, *column),
        models::ComparisonValue::Scalar { value: json_value } => {
            sql_ast::Expression::Value(translate_json_value(&json_value))
        }
        // dummy
        _ => sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
    }
}

fn translate_json_value(value: &serde_json::Value) -> sql_ast::Value {
    match value {
        serde_json::Value::Number(num) => {
            sql_ast::Value::Int4(num.as_i64().unwrap().try_into().unwrap())
        }
        serde_json::Value::Bool(b) => sql_ast::Value::Bool(*b),
        serde_json::Value::String(s) => sql_ast::Value::String(s.to_string()),
        serde_json::Value::Array(items) => {
            let inner_values: Vec<sql_ast::Value> =
                items.iter().map(translate_json_value).collect();
            sql_ast::Value::Array(inner_values)
        }
        // dummy
        _ => sql_ast::Value::Bool(true),
    }
}

/// generate a column expression.
fn make_column(
    table: sql_ast::TableName,
    name: String,
    alias: sql_ast::ColumnAlias,
) -> (sql_ast::ColumnAlias, sql_ast::Expression) {
    (
        alias,
        sql_ast::Expression::ColumnName(sql_ast::ColumnName::TableColumn { table, name }),
    )
}

/// A type for translation errors.
#[derive(Debug)]
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
