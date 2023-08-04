/// Translate the incoming QueryRequest to an ExecutionPlan (SQL) to be run against the database.
/// Also exports the SQL AST types and the low-level string representation of a SQL query type.
pub mod sql;

use crate::metadata;

use indexmap::IndexMap;
use ndc_client::models;

use std::collections::BTreeMap;

#[derive(Debug)]
/// Definition of an execution plan to be run against the database.
pub struct ExecutionPlan {
    pub variables: Option<Vec<BTreeMap<String, serde_json::Value>>>,
    pub root_field: String,
    /// Run before the query. Should be a sql::ast in the future.
    pub pre: Vec<sql::string::DDL>,
    /// The query.
    pub query: sql::ast::Select,
    /// Run after the query. Should be a sql::ast in the future.
    pub post: Vec<sql::string::DDL>,
}

impl ExecutionPlan {
    /// Extract the query component as SQL.
    pub fn query(&self) -> sql::string::SQL {
        select_to_sql(&self.query)
    }
    pub fn explain_query(&self) -> sql::string::SQL {
        explain_to_sql(&sql::ast::Explain::Select(&self.query))
    }
}

pub fn select_to_sql(select: &sql::ast::Select) -> sql::string::SQL {
    let mut sql = sql::string::SQL::new();
    select.to_sql(&mut sql);
    sql
}

pub fn explain_to_sql(explain: &sql::ast::Explain) -> sql::string::SQL {
    let mut sql = sql::string::SQL::new();
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
    query: sql::ast::Select,
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
        let select_set = self.translate_query(
            tables_info,
            &query_request.collection,
            &query_request.collection_relationships,
            query_request.query,
        )?;

        // form a single JSON item shaped `{ rows: [], aggregates: {} }`
        // that matches the models::RowSet type
        let json_select = sql::helpers::select_rowset(
            self.make_column_alias("universe".to_string()),
            self.make_table_alias("universe".to_string()),
            self.make_table_alias("rows".to_string()),
            self.make_column_alias("rows".to_string()),
            self.make_table_alias("aggregates".to_string()),
            self.make_column_alias("aggregates".to_string()),
            select_set,
        );

        // log and return
        tracing::info!("SQL AST: {:?}", json_select);
        Ok(simple_exec_plan(
            query_request.variables,
            query_request.collection,
            json_select,
        ))
    }

    /// Translate a query to sql ast.
    pub fn translate_query(
        &mut self,
        tables_info: &metadata::TablesInfo,
        table_name: &String,
        relationships: &BTreeMap<String, models::Relationship>,
        query: models::Query,
    ) -> Result<sql::helpers::SelectSet, Error> {
        // Error::NoFields becomes Ok(None)
        // everything stays Err
        let map_no_fields_error_to_none = |err| match err {
            Error::NoFields => Ok(None),
            other_error => Err(other_error),
        };

        // wrap valid result in Some
        let wrap_ok = |a| Ok(Some(a));

        // translate rows query. if there are no fields, make this a None
        let row_select: Option<sql::ast::Select> = self
            .translate_rows_query(tables_info, table_name, relationships, &query)
            .map_or_else(map_no_fields_error_to_none, wrap_ok)?;

        // translate aggregate select. if there are no fields, make this a None
        let aggregate_select: Option<sql::ast::Select> = self
            .translate_aggregate_query(tables_info, table_name, relationships, &query)
            .map_or_else(map_no_fields_error_to_none, wrap_ok)?;

        match (row_select, aggregate_select) {
            (Some(rows), None) => Ok(sql::helpers::SelectSet::Rows(rows)),
            (None, Some(aggregates)) => Ok(sql::helpers::SelectSet::Aggregates(aggregates)),
            (Some(rows), Some(aggregates)) => {
                Ok(sql::helpers::SelectSet::RowsAndAggregates(rows, aggregates))
            }
            _ => Err(Error::NoFields),
        }
    }

    /// Translate aggregates query to sql ast.
    pub fn translate_aggregate_query(
        &mut self,
        tables_info: &metadata::TablesInfo,
        table_name: &String,
        relationships: &BTreeMap<String, models::Relationship>,
        query: &models::Query,
    ) -> Result<sql::ast::Select, Error> {
        let metadata::TablesInfo(tables_info_map) = tables_info;
        // find the table according to the metadata.
        let table_info = tables_info_map
            .get(table_name)
            .ok_or(Error::TableNotFound(table_name.clone()))?;
        let table: sql::ast::TableName = sql::ast::TableName::DBTable {
            schema: table_info.schema_name.clone(),
            table: table_info.table_name.clone(),
        };
        let table_alias: sql::ast::TableAlias = self.make_table_alias(table_name.clone());
        let table_alias_name: sql::ast::TableName =
            sql::ast::TableName::AliasedTable(table_alias.clone());

        // join aliases
        let join_fields: Vec<(sql::ast::TableAlias, String, models::Query)> = vec![];

        // translate aggregates to select list
        let aggregate_fields = query.aggregates.clone().ok_or(Error::NoFields)?;

        // fail if no aggregates defined at all
        match IndexMap::is_empty(&aggregate_fields) {
            true => Err(Error::NoFields),
            false => Ok(()),
        }?;

        // create all aggregate columns
        let aggregate_columns = self.translate_aggregates(
            sql::ast::TableName::AliasedTable(table_alias.clone()),
            aggregate_fields,
        )?;

        // construct a simple select with the table name, alias, and selected columns.
        let mut aggregate_select = sql::helpers::simple_select(aggregate_columns);

        aggregate_select.from = Some(sql::ast::From::Table {
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
            &query.order_by,
        )?;

        relationship_joins.extend(order_by_joins);

        aggregate_select.joins = relationship_joins;

        aggregate_select.order_by = order_by;

        // translate where
        aggregate_select.where_ = sql::ast::Where(match query.clone().predicate {
            None => sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
            Some(predicate) => self.translate_expression(&table_alias_name, predicate),
        });

        Ok(aggregate_select)
    }

    /// Translate rows part of query to sql ast.
    pub fn translate_rows_query(
        &mut self,
        tables_info: &metadata::TablesInfo,
        table_name: &String,
        relationships: &BTreeMap<String, models::Relationship>,
        query: &models::Query,
    ) -> Result<sql::ast::Select, Error> {
        let metadata::TablesInfo(tables_info_map) = tables_info;
        // find the table according to the metadata.
        let table_info = tables_info_map
            .get(table_name)
            .ok_or(Error::TableNotFound(table_name.clone()))?;
        let table: sql::ast::TableName = sql::ast::TableName::DBTable {
            schema: table_info.schema_name.clone(),
            table: table_info.table_name.clone(),
        };
        let table_alias: sql::ast::TableAlias = self.make_table_alias(table_name.clone());
        let table_alias_name: sql::ast::TableName =
            sql::ast::TableName::AliasedTable(table_alias.clone());

        // join aliases
        let mut join_fields: Vec<(sql::ast::TableAlias, String, models::Query)> = vec![];

        // translate fields to select list
        let fields = query.fields.clone().ok_or(Error::NoFields)?;

        // fail if no columns defined at all
        match IndexMap::is_empty(&fields) {
            true => Err(Error::NoFields),
            false => Ok(()),
        }?;

        // translate fields to columns or relationships.
        let columns: Vec<(sql::ast::ColumnAlias, sql::ast::Expression)> = fields
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
                    let column_name = sql::ast::ColumnName::AliasedColumn {
                        table: sql::ast::TableName::AliasedTable(table_alias.clone()),
                        name: column_alias.clone(),
                    };
                    join_fields.push((table_alias, relationship, *query));
                    Ok((column_alias, sql::ast::Expression::ColumnName(column_name)))
                }
            })
            .collect::<Result<Vec<_>, Error>>()?;

        // construct a simple select with the table name, alias, and selected columns.
        let mut rows_select = sql::helpers::simple_select(columns);

        rows_select.from = Some(sql::ast::From::Table {
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
            &query.order_by,
        )?;

        relationship_joins.extend(order_by_joins);

        rows_select.joins = relationship_joins;

        rows_select.order_by = order_by;

        // translate where
        rows_select.where_ = sql::ast::Where(match query.predicate.clone() {
            None => sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
            Some(predicate) => self.translate_expression(&table_alias_name, predicate),
        });

        // translate limit and offset
        rows_select.limit = sql::ast::Limit {
            limit: query.limit,
            offset: query.offset,
        };

        Ok(rows_select)
    }

    /// create column aliases using this function so they get a unique index.
    fn make_column_alias(&mut self, name: String) -> sql::ast::ColumnAlias {
        let index = self.unique_index;
        self.unique_index += 1;
        sql::ast::ColumnAlias {
            unique_index: index,
            name,
        }
    }
    /// create table aliases using this function so they get a unique index.
    fn make_table_alias(&mut self, name: String) -> sql::ast::TableAlias {
        let index = self.unique_index;
        self.unique_index += 1;
        sql::ast::TableAlias {
            unique_index: index,
            name,
        }
    }

    // given a relationship, turn it into a Where clause for a Join
    fn translate_column_mapping(
        &mut self,
        tables_info: &metadata::TablesInfo,
        table_name: &str,
        table_alias: &sql::ast::TableAlias,
        expr: sql::ast::Expression,
        relationship: &models::Relationship,
    ) -> Result<sql::ast::Expression, Error> {
        let metadata::TablesInfo(tables_info_map) = tables_info;

        let table_info = tables_info_map
            .get(table_name)
            .ok_or(Error::TableNotFound(table_name.to_string()))?;

        let target_collection_info = tables_info_map
            .get(&relationship.target_collection)
            .ok_or(Error::TableNotFound(relationship.target_collection.clone()))?;
        let target_collection_alias: sql::ast::TableAlias =
            self.make_table_alias(relationship.target_collection.clone());
        let target_collection_alias_name: sql::ast::TableName =
            sql::ast::TableName::AliasedTable(target_collection_alias);

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
                let target_column_info = target_collection_info.columns.get(target_col).ok_or(
                    Error::ColumnNotFoundInTable(
                        target_col.clone(),
                        relationship.target_collection.clone(),
                    ),
                )?;
                Ok(sql::ast::Expression::BinaryOperator {
                    left: Box::new(sql::ast::Expression::ColumnName(
                        sql::ast::ColumnName::TableColumn {
                            table: sql::ast::TableName::AliasedTable(table_alias.clone()),
                            name: source_column_info.name.clone(),
                        },
                    )),
                    operator: sql::ast::BinaryOperator::Equals,
                    right: Box::new(sql::ast::Expression::ColumnName(
                        sql::ast::ColumnName::TableColumn {
                            table: target_collection_alias_name.clone(),
                            name: target_column_info.name.clone(),
                        },
                    )),
                })
            })
            .try_fold(expr, |expr, op| {
                let op = op?;
                Ok(sql::ast::Expression::And {
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
        table_alias: &sql::ast::TableAlias,
        table_name: &str,
        join_fields: Vec<(sql::ast::TableAlias, String, models::Query)>,
    ) -> Result<Vec<sql::ast::Join>, Error> {
        join_fields
            .into_iter()
            .map(|(alias, relationship_name, query)| {
                let relationship = relationships
                    .get(&relationship_name)
                    .ok_or(Error::RelationshipNotFound(relationship_name.clone()))?;

                let mut select = self.translate_rows_query(
                    tables_info,
                    &relationship.target_collection,
                    relationships,
                    &query,
                )?;

                // apply join conditions
                let sql::ast::Where(expr) = select.where_;

                let with_join_condition = self.translate_column_mapping(
                    tables_info,
                    table_name,
                    table_alias,
                    expr,
                    relationship,
                )?;

                select.where_ = sql::ast::Where(with_join_condition);

                // when we want to get nested aggregates working, we should be using
                // `select_rowset` here instead so that we also generate selects for any aggregate
                // rows
                // we'll need to work out a way of generating unique table aliases that don't
                // collide with the top level ones first though
                let wrap_select = match relationship.relationship_type {
                    // for some reason v3-engine expects object relationships
                    // also in the form of a json array wrapped in `rows`.
                    models::RelationshipType::Object => {
                        sql::helpers::select_table_as_json_array_in_rows_object
                    }
                    models::RelationshipType::Array => {
                        sql::helpers::select_table_as_json_array_in_rows_object
                    }
                };

                // wrap the sql in row_to_json and json_agg
                let final_select = wrap_select(
                    select,
                    self.make_column_alias(alias.name.clone()),
                    self.make_table_alias(alias.name.clone()),
                );

                Ok(sql::ast::Join::LeftOuterJoinLateral(
                    sql::ast::LeftOuterJoinLateral {
                        select: Box::new(final_select),
                        alias,
                    },
                ))
            })
            .collect::<Result<Vec<sql::ast::Join>, Error>>()
    }

    // translate any aggregates we should include in the query into our SQL AST
    fn translate_aggregates(
        &mut self,
        table: sql::ast::TableName,
        aggregates: IndexMap<String, models::Aggregate>,
    ) -> Result<Vec<(sql::ast::ColumnAlias, sql::ast::Expression)>, Error> {
        aggregates
            .into_iter()
            .map(|(alias, aggregation)| {
                let expression = match aggregation {
                    models::Aggregate::ColumnCount { column, distinct } => {
                        let count_column_alias = self.make_column_alias(column);
                        if distinct {
                            sql::ast::Expression::Count(sql::ast::CountType::Distinct(
                                sql::ast::ColumnName::AliasedColumn {
                                    table: table.clone(),
                                    name: count_column_alias,
                                },
                            ))
                        } else {
                            sql::ast::Expression::Count(sql::ast::CountType::Simple(
                                sql::ast::ColumnName::AliasedColumn {
                                    table: table.clone(),
                                    name: count_column_alias,
                                },
                            ))
                        }
                    }
                    models::Aggregate::SingleColumn { column, function } => {
                        sql::ast::Expression::FunctionCall {
                            function: sql::ast::Function::Unknown(function),
                            args: vec![sql::ast::Expression::ColumnName(
                                sql::ast::ColumnName::AliasedColumn {
                                    table: table.clone(),
                                    name: self.make_column_alias(column),
                                },
                            )],
                        }
                    }
                    models::Aggregate::StarCount {} => {
                        sql::ast::Expression::Count(sql::ast::CountType::Star)
                    }
                };
                Ok((self.make_column_alias(alias), expression))
            })
            .collect::<Result<Vec<_>, Error>>()
    }

    // generate expression and joins for ordering by a column
    fn translate_order_by_target_for_column(
        &mut self,
        tables_info: &metadata::TablesInfo,
        relationships: &BTreeMap<String, models::Relationship>,
        initial_table_alias: &sql::ast::TableAlias,
        raw_table_name: &String,
        name: &String,
        path: &[models::PathElement],
    ) -> Result<(sql::ast::Expression, Vec<sql::ast::Join>), Error> {
        let mut joins: Vec<sql::ast::Join> = vec![];

        // start with local column
        let initial_order_by_expression =
            sql::ast::Expression::ColumnName(sql::ast::ColumnName::AliasedColumn {
                table: sql::ast::TableName::AliasedTable(initial_table_alias.clone()),
                name: self.make_column_alias(name.to_string()),
            });

        // loop through relationships, building up new joins and replacing the order_by_expression
        let (order_by_expression, _) = path.iter().enumerate().try_fold(
            (initial_order_by_expression, raw_table_name),
            |inputs, path_element| {
                let (_, table_name) = inputs;
                let (
                    index,
                    models::PathElement {
                        relationship: relationship_name,
                        ..
                    },
                ) = path_element;

                let relationship = relationships
                    .get(relationship_name)
                    .ok_or(Error::RelationshipNotFound(relationship_name.clone()))?;

                match relationship.relationship_type {
                    models::RelationshipType::Array => Err(Error::NotSupported(
                        "Cannot order by values in an array relationship".to_string(),
                    )),
                    models::RelationshipType::Object => {
                        let table_alias: sql::ast::TableAlias =
                            self.make_table_alias(table_name.to_string());

                        let target_collection_alias: sql::ast::TableAlias =
                            self.make_table_alias(relationship.target_collection.clone());

                        let target_collection_alias_name: sql::ast::TableName =
                            sql::ast::TableName::AliasedTable(target_collection_alias.clone());

                        // we must include any rows referenced in the join
                        let mut join_rows: Vec<(sql::ast::ColumnAlias, sql::ast::Expression)> =
                            relationship
                                .column_mapping
                                .values()
                                .map(|target_col| {
                                    let new_column_alias =
                                        self.make_column_alias(target_col.to_string());
                                    let new_table = target_collection_alias_name.clone();
                                    (
                                        new_column_alias.clone(),
                                        sql::ast::Expression::ColumnName(
                                            sql::ast::ColumnName::AliasedColumn {
                                                table: new_table,
                                                name: new_column_alias,
                                            },
                                        ),
                                    )
                                })
                                .collect();

                        // AND any rows referenced in the "next" join, if there is one
                        match path.get(index + 1) {
                            Some(models::PathElement {
                                relationship: next_relationship_name,
                                ..
                            }) => {
                                let next_relationship =
                                    relationships.get(next_relationship_name).ok_or(
                                        Error::RelationshipNotFound(next_relationship_name.clone()),
                                    )?;
                                let more_rows: Vec<_> = next_relationship
                                    .column_mapping
                                    .keys()
                                    .map(|source_col| {
                                        let new_column_alias =
                                            self.make_column_alias(source_col.to_string());
                                        let new_table = target_collection_alias_name.clone();
                                        (
                                            new_column_alias.clone(),
                                            sql::ast::Expression::ColumnName(
                                                sql::ast::ColumnName::AliasedColumn {
                                                    table: new_table,
                                                    name: new_column_alias,
                                                },
                                            ),
                                        )
                                    })
                                    .collect();
                                join_rows.extend(more_rows);
                            }
                            None => {
                                // if there's no "next" join, this is the one with the value we're
                                // actually ordering by, so we need to make sure it's included
                                join_rows.push((
                                    self.make_column_alias(name.to_string()),
                                    sql::ast::Expression::ColumnName(
                                        sql::ast::ColumnName::AliasedColumn {
                                            table: target_collection_alias_name.clone(),
                                            name: self.make_column_alias(name.to_string()),
                                        },
                                    ),
                                ));
                            }
                        };

                        // select those rows pls
                        let mut select = sql::helpers::simple_select(join_rows);

                        // generate a condition for this join
                        let join_condition = self.translate_column_mapping(
                            tables_info,
                            table_name,
                            &table_alias,
                            sql::helpers::empty_where(),
                            relationship,
                        )?;

                        select.where_ = sql::ast::Where(join_condition);

                        select.from = Some(sql::ast::From::Table {
                            name: target_collection_alias_name.clone(),
                            alias: target_collection_alias.clone(),
                        });

                        let join =
                            sql::ast::Join::LeftOuterJoinLateral(sql::ast::LeftOuterJoinLateral {
                                select: Box::new(select),
                                alias: target_collection_alias,
                            });

                        // add the join to our pile'o'joins
                        joins.push(join);

                        Ok((
                            sql::ast::Expression::ColumnName(sql::ast::ColumnName::AliasedColumn {
                                table: target_collection_alias_name,
                                name: self.make_column_alias(name.to_string()),
                            }),
                            &relationship.target_collection,
                        ))
                    }
                }
            },
        )?;

        Ok((order_by_expression, joins))
    }

    /// Convert the order by fields from a QueryRequest to a SQL ORDER BY clause and potentially
    /// JOINs when we order by relationship fields.
    fn translate_order_by(
        &mut self,
        tables_info: &metadata::TablesInfo,
        relationships: &BTreeMap<String, models::Relationship>,
        source_table_alias: &sql::ast::TableAlias,
        source_table_name: &String,
        order_by: &Option<models::OrderBy>,
    ) -> Result<(sql::ast::OrderBy, Vec<sql::ast::Join>), Error> {
        let mut joins: Vec<sql::ast::Join> = vec![];

        // For each order_by field, extract the relevant field name, direction, and join table (if relevant).
        match order_by {
            None => Ok((sql::ast::OrderBy { elements: vec![] }, vec![])),
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
                                        source_table_alias,
                                        source_table_name,
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
                            models::OrderDirection::Asc => sql::ast::OrderByDirection::Asc,
                            models::OrderDirection::Desc => sql::ast::OrderByDirection::Desc,
                        };
                        Ok(sql::ast::OrderByElement { target, direction })
                    })
                    .collect::<Result<Vec<sql::ast::OrderByElement>, Error>>()?;

                Ok((
                    sql::ast::OrderBy {
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
        table: &sql::ast::TableName,
        predicate: models::Expression,
    ) -> sql::ast::Expression {
        match predicate {
            models::Expression::And { expressions } => expressions
                .into_iter()
                .map(|expr| self.translate_expression(table, expr))
                .fold(
                    sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
                    |acc, expr| sql::ast::Expression::And {
                        left: Box::new(acc),
                        right: Box::new(expr),
                    },
                ),
            models::Expression::Or { expressions } => expressions
                .into_iter()
                .map(|expr| self.translate_expression(table, expr))
                .fold(
                    sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
                    |acc, expr| sql::ast::Expression::Or {
                        left: Box::new(acc),
                        right: Box::new(expr),
                    },
                ),
            models::Expression::Not { expression } => {
                sql::ast::Expression::Not(Box::new(self.translate_expression(table, *expression)))
            }
            models::Expression::BinaryComparisonOperator {
                column,
                operator,
                value,
            } => sql::ast::Expression::BinaryOperator {
                left: Box::new(translate_comparison_target(table, *column)),
                operator: match *operator {
                    models::BinaryComparisonOperator::Equal => sql::ast::BinaryOperator::Equals,
                    models::BinaryComparisonOperator::Other { name } =>
                    // the strings we're matching against here (ie 'like') are best guesses for now, will
                    // need to update these as find out more
                    {
                        match &*name {
                            "like" => sql::ast::BinaryOperator::Like,
                            "nlike" => sql::ast::BinaryOperator::NotLike,
                            "ilike" => sql::ast::BinaryOperator::CaseInsensitiveLike,
                            "nilike" => sql::ast::BinaryOperator::NotCaseInsensitiveLike,
                            "similar" => sql::ast::BinaryOperator::Similar,
                            "nsimilar" => sql::ast::BinaryOperator::NotSimilar,
                            "regex" => sql::ast::BinaryOperator::Regex,
                            "nregex" => sql::ast::BinaryOperator::NotRegex,
                            "iregex" => sql::ast::BinaryOperator::CaseInsensitiveRegex,
                            "niregex" => sql::ast::BinaryOperator::NotCaseInsensitiveRegex,
                            "lt" => sql::ast::BinaryOperator::LessThan,
                            "lte" => sql::ast::BinaryOperator::LessThanOrEqualTo,
                            "gt" => sql::ast::BinaryOperator::GreaterThan,
                            "gte" => sql::ast::BinaryOperator::GreaterThanOrEqualTo,
                            _ => sql::ast::BinaryOperator::Equals,
                        }
                    }
                },
                right: Box::new(translate_comparison_value(table, *value)),
            },
            models::Expression::BinaryArrayComparisonOperator {
                column,
                operator,
                values,
            } => sql::ast::Expression::BinaryArrayOperator {
                left: Box::new(translate_comparison_target(table, *column)),
                operator: match *operator {
                    models::BinaryArrayComparisonOperator::In => sql::ast::BinaryArrayOperator::In,
                },
                right: values
                    .iter()
                    .map(|value| translate_comparison_value(table, value.clone()))
                    .collect(),
            },

            // dummy
            _ => sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
        }
    }
}
/// translate a comparison target.
fn translate_comparison_target(
    table: &sql::ast::TableName,
    column: models::ComparisonTarget,
) -> sql::ast::Expression {
    match column {
        models::ComparisonTarget::Column { name, .. } => {
            sql::ast::Expression::ColumnName(sql::ast::ColumnName::TableColumn {
                table: table.clone(),
                name,
            })
        }
        // dummy
        _ => sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
    }
}

/// translate a comparison value.
fn translate_comparison_value(
    table: &sql::ast::TableName,
    value: models::ComparisonValue,
) -> sql::ast::Expression {
    match value {
        models::ComparisonValue::Column { column } => translate_comparison_target(table, *column),
        models::ComparisonValue::Scalar { value: json_value } => {
            sql::ast::Expression::Value(translate_json_value(&json_value))
        }
        models::ComparisonValue::Variable { name: var } => {
            sql::ast::Expression::Value(sql::ast::Value::Variable(var))
        }
    }
}

fn translate_json_value(value: &serde_json::Value) -> sql::ast::Value {
    match value {
        serde_json::Value::Number(num) => {
            sql::ast::Value::Int4(num.as_i64().unwrap().try_into().unwrap())
        }
        serde_json::Value::Bool(b) => sql::ast::Value::Bool(*b),
        serde_json::Value::String(s) => sql::ast::Value::String(s.to_string()),
        serde_json::Value::Array(items) => {
            let inner_values: Vec<sql::ast::Value> =
                items.iter().map(translate_json_value).collect();
            sql::ast::Value::Array(inner_values)
        }
        // dummy
        _ => sql::ast::Value::Bool(true),
    }
}

/// generate a column expression.
fn make_column(
    table: sql::ast::TableName,
    name: String,
    alias: sql::ast::ColumnAlias,
) -> (sql::ast::ColumnAlias, sql::ast::Expression) {
    (
        alias,
        sql::ast::Expression::ColumnName(sql::ast::ColumnName::TableColumn { table, name }),
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
