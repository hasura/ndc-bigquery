/// Translate the incoming QueryRequest to an ExecutionPlan (SQL) to be run against the database.
/// Also exports the SQL AST types and the low-level string representation of a SQL query type.
pub mod convert;
pub mod sql_ast;
pub mod sql_helpers;
pub mod sql_string;
use crate::metadata;

use indexmap::IndexMap;
use ndc_client::models;

use std::collections::BTreeMap;

#[derive(Debug)]
/// Definition of an execution plan to be run against the database.
pub struct ExecutionPlan {
    pub variables: Option<Vec<BTreeMap<String, serde_json::Value>>>,
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
    pub fn explain_query(&self) -> sql_string::SQL {
        explain_to_sql(&sql_ast::Explain::Select(&self.query))
    }
}

pub fn select_to_sql(select: &sql_ast::Select) -> sql_string::SQL {
    let mut sql = sql_string::SQL::new();
    select.to_sql(&mut sql);
    sql
}

pub fn explain_to_sql(explain: &sql_ast::Explain) -> sql_string::SQL {
    let mut sql = sql_string::SQL::new();
    explain.to_sql(&mut sql);
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
pub fn simple_exec_plan(
    variables: Option<Vec<BTreeMap<String, serde_json::Value>>>,
    root_field: String,
    query: sql_ast::Select,
) -> ExecutionPlan {
    ExecutionPlan {
        variables,
        root_field,
        pre: vec![],
        query,
        post: vec![],
    }
}

/// State for the translation phase
pub struct Translate {
    /// give each alias a unique name using this number.
    unique_index: u64,
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
        tables_info: &metadata::TablesInfo,
        query_request: models::QueryRequest,
    ) -> Result<ExecutionPlan, Error> {
        let select = self.translate_query(
            tables_info,
            &query_request.collection,
            &query_request.collection_relationships,
            query_request.query,
        )?;

        // wrap the sql in row_to_json and json_agg
        let final_select = sql_helpers::select_table_as_json_array(
            select,
            self.make_column_alias("rows".to_string()),
            self.make_table_alias("root".to_string()),
        );

        // log and return
        tracing::info!("SQL AST: {:?}", final_select);
        Ok(simple_exec_plan(
            query_request.variables,
            query_request.collection,
            final_select,
        ))
    }

    /// Translate a query to sql ast.
    pub fn translate_query(
        &mut self,
        tables_info: &metadata::TablesInfo,
        table_name: &String,
        relationships: &BTreeMap<String, models::Relationship>,
        query: models::Query,
    ) -> Result<sql_ast::Select, Error> {
        let metadata::TablesInfo(tables_info_map) = tables_info;
        // find the table according to the metadata.
        let table_info = tables_info_map
            .get(table_name)
            .ok_or(Error::TableNotFound(table_name.clone()))?;
        let table: sql_ast::TableName = sql_ast::TableName::DBTable {
            schema: table_info.schema_name.clone(),
            table: table_info.table_name.clone(),
        };
        let table_alias: sql_ast::TableAlias = self.make_table_alias(table_name.clone());
        let table_alias_name: sql_ast::TableName =
            sql_ast::TableName::AliasedTable(table_alias.clone());

        // join aliases
        let mut join_fields: Vec<(sql_ast::TableAlias, String, models::Query)> = vec![];

        // translate fields to select list
        let fields = query.fields.unwrap_or(IndexMap::new());

        // translate fields to columns or relationships.
        let mut columns: Vec<(sql_ast::ColumnAlias, sql_ast::Expression)> = fields
            .into_iter()
            .map(|(alias, field)| match field {
                models::Field::Column { column, .. } => {
                    let column_info = table_info
                        .columns
                        .get(&column)
                        .ok_or(Error::ColumnNotFoundInTable(column, table_name.clone()))?;
                    Ok(make_column(
                        table_alias_name.clone(),
                        column_info.name.clone(),
                        self.make_column_alias(alias),
                    ))
                }
                models::Field::Relationship {
                    query,
                    relationship,
                    ..
                } => {
                    let table_alias = self.make_table_alias(alias.clone());
                    let column_alias = self.make_column_alias(alias);
                    let column_name = sql_ast::ColumnName::AliasedColumn {
                        table: sql_ast::TableName::AliasedTable(table_alias.clone()),
                        name: column_alias.clone(),
                    };
                    join_fields.push((table_alias, relationship, *query));
                    Ok((column_alias, sql_ast::Expression::ColumnName(column_name)))
                }
            })
            .collect::<Result<Vec<_>, Error>>()?;

        // create all aggregate columns
        let aggregate_columns = self.translate_aggregates(
            sql_ast::TableName::AliasedTable(table_alias.clone()),
            query.aggregates.unwrap_or(IndexMap::new()),
        )?;

        // combine field and aggregate columns
        columns.extend(aggregate_columns);

        // fail if no columns defined at all
        match Vec::is_empty(&columns) {
            true => Err(Error::NoFields),
            false => Ok(()),
        }?;

        // construct a simple select with the table name, alias, and selected columns.
        let mut select = sql_helpers::simple_select(columns);

        select.from = Some(sql_ast::From::Table {
            name: table,
            alias: table_alias.clone(),
        });

        // collect any joins for relationships
        let mut relationship_joins = self.translate_joins(
            relationships,
            tables_info,
            &table_alias,
            table_name,
            join_fields,
        )?;

        // translate order_by
        let (order_by, order_by_joins) = self.translate_order_by(
            tables_info,
            relationships,
            &table_alias,
            table_name,
            query.order_by,
        )?;

        relationship_joins.extend(order_by_joins);

        select.joins = relationship_joins;

        select.order_by = order_by;

        // translate where
        select.where_ = sql_ast::Where(match query.predicate {
            None => sql_ast::Expression::Value(sql_ast::Value::Bool(true)),
            Some(predicate) => self.translate_expression(&table_alias_name, predicate),
        });

        // translate limit and offset
        select.limit = sql_ast::Limit {
            limit: query.limit,
            offset: query.offset,
        };

        Ok(select)
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

    // given a relationship, turn it into a Where clause for a Join
    fn translate_column_mapping(
        &mut self,
        tables_info: &metadata::TablesInfo,
        table_info: &metadata::TableInfo,
        table_name: &str,
        table_alias: &sql_ast::TableAlias,
        expr: sql_ast::Expression,
        relationship: &models::Relationship,
    ) -> Result<sql_ast::Expression, Error> {
        let metadata::TablesInfo(tables_info_map) = tables_info;

        let target_table_info = tables_info_map
            .get(&relationship.target_collection)
            .ok_or(Error::TableNotFound(relationship.target_collection.clone()))?;
        let target_table_alias: sql_ast::TableAlias =
            self.make_table_alias(relationship.target_collection.clone());
        let target_table_alias_name: sql_ast::TableName =
            sql_ast::TableName::AliasedTable(target_table_alias);

        relationship
            .column_mapping
            .iter()
            .map(|(source_col, target_col)| {
                let source_column_info =
                    table_info
                        .columns
                        .get(source_col)
                        .ok_or(Error::ColumnNotFoundInTable(
                            source_col.clone(),
                            table_name.to_string(),
                        ))?;
                let target_column_info = target_table_info.columns.get(target_col).ok_or(
                    Error::ColumnNotFoundInTable(
                        target_col.clone(),
                        relationship.target_collection.clone(),
                    ),
                )?;
                Ok(sql_ast::Expression::BinaryOperator {
                    left: Box::new(sql_ast::Expression::ColumnName(
                        sql_ast::ColumnName::TableColumn {
                            table: sql_ast::TableName::AliasedTable(table_alias.clone()),
                            name: source_column_info.name.clone(),
                        },
                    )),
                    operator: sql_ast::BinaryOperator::Equals,
                    right: Box::new(sql_ast::Expression::ColumnName(
                        sql_ast::ColumnName::TableColumn {
                            table: target_table_alias_name.clone(),
                            name: target_column_info.name.clone(),
                        },
                    )),
                })
            })
            .try_fold(expr, |expr, op| {
                let op = op?;
                Ok(sql_ast::Expression::And {
                    left: Box::new(expr),
                    right: Box::new(op),
                })
            })
    }

    // translate any joins we should include in the query into our SQL AST
    fn translate_joins(
        &mut self,
        relationships: &BTreeMap<String, models::Relationship>,
        tables_info: &metadata::TablesInfo,
        table_alias: &sql_ast::TableAlias,
        table_name: &str,
        join_fields: Vec<(sql_ast::TableAlias, String, models::Query)>,
    ) -> Result<Vec<sql_ast::Join>, Error> {
        let metadata::TablesInfo(tables_info_map) = tables_info;
        // find the table according to the metadata.
        let table_info = tables_info_map
            .get(table_name)
            .ok_or(Error::TableNotFound(table_name.to_string()))?;

        join_fields
            .into_iter()
            .map(|(alias, relationship_name, query)| {
                let relationship = relationships
                    .get(&relationship_name)
                    .ok_or(Error::RelationshipNotFound(relationship_name.clone()))?;

                let mut select = self.translate_query(
                    tables_info,
                    &relationship.target_collection,
                    relationships,
                    query,
                )?;

                // apply join conditions
                let sql_ast::Where(expr) = select.where_;

                let with_join_condition = self.translate_column_mapping(
                    tables_info,
                    table_info,
                    table_name,
                    table_alias,
                    expr,
                    relationship,
                )?;

                select.where_ = sql_ast::Where(with_join_condition);

                let wrap_select = match relationship.relationship_type {
                    // objects should return `sql_helpers::select_row_as_json`, but we need an
                    // array so that we turn it into a `Relationship` in `response_hack.rs`
                    models::RelationshipType::Object => sql_helpers::select_table_as_json_array,
                    models::RelationshipType::Array => sql_helpers::select_table_as_json_array,
                };

                // wrap the sql in row_to_json and json_agg
                let final_select = wrap_select(
                    select,
                    self.make_column_alias(alias.name.clone()),
                    self.make_table_alias(alias.name.clone()),
                );

                Ok(sql_ast::Join::LeftOuterJoinLateral(
                    sql_ast::LeftOuterJoinLateral {
                        select: Box::new(final_select),
                        alias,
                    },
                ))
            })
            .collect::<Result<Vec<sql_ast::Join>, Error>>()
    }

    // translate any aggregates we should include in the query into our SQL AST
    fn translate_aggregates(
        &mut self,
        table: sql_ast::TableName,
        aggregates: IndexMap<String, models::Aggregate>,
    ) -> Result<Vec<(sql_ast::ColumnAlias, sql_ast::Expression)>, Error> {
        aggregates
            .into_iter()
            .map(|(alias, aggregation)| {
                let expression = match aggregation {
                    models::Aggregate::ColumnCount { column, distinct } => {
                        let count_column_alias = self.make_column_alias(column);
                        if distinct {
                            sql_ast::Expression::Count(sql_ast::CountType::Distinct(
                                sql_ast::ColumnName::AliasedColumn {
                                    table: table.clone(),
                                    name: count_column_alias,
                                },
                            ))
                        } else {
                            sql_ast::Expression::Count(sql_ast::CountType::Simple(
                                sql_ast::ColumnName::AliasedColumn {
                                    table: table.clone(),
                                    name: count_column_alias,
                                },
                            ))
                        }
                    }
                    models::Aggregate::SingleColumn { column, function } => {
                        sql_ast::Expression::FunctionCall {
                            function: sql_ast::Function::Unknown(function),
                            args: vec![sql_ast::Expression::ColumnName(
                                sql_ast::ColumnName::AliasedColumn {
                                    table: table.clone(),
                                    name: self.make_column_alias(column),
                                },
                            )],
                        }
                    }
                    models::Aggregate::StarCount {} => {
                        sql_ast::Expression::Count(sql_ast::CountType::Star)
                    }
                };
                Ok((self.make_column_alias(alias), expression))
            })
            .collect::<Result<Vec<_>, Error>>()
    }

    // generate expression and joins for ordering by a column
    // currently only allows 0 or 1 levels of relationship
    fn translate_order_by_target_for_column(
        &mut self,
        tables_info: &metadata::TablesInfo,
        relationships: &BTreeMap<String, models::Relationship>,
        table_alias: &sql_ast::TableAlias,
        table_name: &String,
        name: &String,
        path: &Vec<models::PathElement>,
    ) -> Result<(sql_ast::Expression, Vec<sql_ast::Join>), Error> {
        let table = sql_ast::TableName::AliasedTable(table_alias.clone());
        let mut joins: Vec<sql_ast::Join> = vec![];

        let metadata::TablesInfo(tables_info_map) = tables_info;

        if path.is_empty() {
            Ok((
                sql_ast::Expression::ColumnName(sql_ast::ColumnName::AliasedColumn {
                    table,
                    name: self.make_column_alias(name.to_string()),
                }),
                joins,
            ))
        } else {
            match path.get(0) {
                Some(models::PathElement {
                    relationship: relationship_name,
                    ..
                }) => {
                    let relationship = relationships
                        .get(relationship_name)
                        .ok_or(Error::RelationshipNotFound(relationship_name.clone()))?;

                    match relationship.relationship_type {
                        models::RelationshipType::Array => Err(Error::NotSupported(
                            "Cannot order by values in an array relationship".to_string(),
                        )),
                        models::RelationshipType::Object => {
                            let table_info = tables_info_map
                                .get(table_name)
                                .ok_or(Error::TableNotFound(table_name.clone()))?;

                            let target_table_alias: sql_ast::TableAlias =
                                self.make_table_alias(relationship.target_collection.clone());

                            let target_table_alias_name: sql_ast::TableName =
                                sql_ast::TableName::AliasedTable(target_table_alias.clone());

                            let select_expression = sql_ast::Expression::ColumnName(
                                sql_ast::ColumnName::AliasedColumn {
                                    table: target_table_alias_name.clone(),
                                    name: self.make_column_alias(name.to_string()),
                                },
                            );

                            let row = (self.make_column_alias(name.to_string()), select_expression);

                            let mut select = sql_helpers::simple_select(vec![row]);

                            let join_condition = self.translate_column_mapping(
                                tables_info,
                                table_info,
                                table_name,
                                table_alias,
                                sql_helpers::empty_where(),
                                relationship,
                            )?;

                            select.where_ = sql_ast::Where(join_condition);

                            select.from = Some(sql_ast::From::Table {
                                name: target_table_alias_name.clone(),
                                alias: target_table_alias.clone(),
                            });

                            let join = sql_ast::Join::LeftOuterJoinLateral(
                                sql_ast::LeftOuterJoinLateral {
                                    select: Box::new(select),
                                    alias: target_table_alias,
                                },
                            );

                            joins.push(join);

                            Ok((
                                sql_ast::Expression::ColumnName(
                                    sql_ast::ColumnName::AliasedColumn {
                                        table: target_table_alias_name,
                                        name: self.make_column_alias(name.to_string()),
                                    },
                                ),
                                joins,
                            ))
                        }
                    }
                }
                None => Err(Error::NotSupported(
                    "order by nested relationship values".to_string(),
                )),
            }
        }
    }

    fn translate_order_by(
        &mut self,
        tables_info: &metadata::TablesInfo,
        relationships: &BTreeMap<String, models::Relationship>,
        table_alias: &sql_ast::TableAlias,
        table_name: &String,
        order_by: Option<models::OrderBy>,
    ) -> Result<(sql_ast::OrderBy, Vec<sql_ast::Join>), Error> {
        let mut joins: Vec<sql_ast::Join> = vec![];

        match order_by {
            None => Ok((sql_ast::OrderBy { elements: vec![] }, joins)),
            Some(models::OrderBy { elements }) => {
                let order_by_parts = elements
                    .iter()
                    .map(|order_by| {
                        let target = match &order_by.target {
                            models::OrderByTarget::Column { name, path } => {
                                let (expression, new_joins) = self
                                    .translate_order_by_target_for_column(
                                        tables_info,
                                        relationships,
                                        table_alias,
                                        table_name,
                                        name,
                                        path,
                                    )?;

                                joins.extend(new_joins);

                                Ok(expression)
                            }

                            models::OrderByTarget::SingleColumnAggregate { .. } => Err(
                                Error::NotSupported("order by column aggregates".to_string()),
                            ),
                            models::OrderByTarget::StarCountAggregate { .. } => Err(
                                Error::NotSupported("order by star count aggregates".to_string()),
                            ),
                        }?;
                        let direction = match order_by.order_direction {
                            models::OrderDirection::Asc => sql_ast::OrderByDirection::Asc,
                            models::OrderDirection::Desc => sql_ast::OrderByDirection::Desc,
                        };
                        Ok(sql_ast::OrderByElement { target, direction })
                    })
                    .collect::<Result<Vec<sql_ast::OrderByElement>, Error>>()?;
                Ok((
                    sql_ast::OrderBy {
                        elements: order_by_parts,
                    },
                    joins,
                ))
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn translate_expression(
        &mut self,
        table: &sql_ast::TableName,
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
        models::ComparisonValue::Variable { name: var } => {
            sql_ast::Expression::Value(sql_ast::Value::Variable(var))
        }
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
pub enum Error {
    TableNotFound(String),
    ColumnNotFoundInTable(String, String),
    RelationshipNotFound(String),
    NoFields,
    NotSupported(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::TableNotFound(table_name) => write!(f, "Table '{}' not found.", table_name),
            Error::ColumnNotFoundInTable(column_name, table_name) => write!(
                f,
                "Column '{}' not found in table '{}'.",
                column_name, table_name
            ),
            Error::RelationshipNotFound(relationship_name) => {
                write!(f, "Relationship '{}' not found.", relationship_name)
            }
            Error::NotSupported(thing) => {
                write!(f, "Queries containing {} are not supported.", thing)
            }
            Error::NoFields => {
                write!(f, "No fields in request.")
            }
        }
    }
}
