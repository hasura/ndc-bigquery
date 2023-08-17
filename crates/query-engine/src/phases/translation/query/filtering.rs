//! Handle filtering/where clauses translation.

use ndc_hub::models;

use super::error::Error;
use super::helpers::{Env, RootAndCurrentTables, TableNameAndReference};
use super::relationships;
use crate::metadata;
use crate::phases::translation::sql;
use crate::phases::translation::sql::helpers::simple_select;

/// Translate a boolean expression to a SQL expression.
pub fn translate_expression(
    env: &Env,
    root_and_current_tables: &RootAndCurrentTables,
    predicate: models::Expression,
) -> Result<sql::ast::Expression, Error> {
    match predicate {
        models::Expression::And { expressions } => expressions
            .into_iter()
            .map(|expr| translate_expression(env, root_and_current_tables, expr))
            .try_fold(
                sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
                |acc, expr| {
                    let right = expr?;
                    Ok(sql::ast::Expression::And {
                        left: Box::new(acc),
                        right: Box::new(right),
                    })
                },
            ),
        models::Expression::Or { expressions } => expressions
            .into_iter()
            .map(|expr| translate_expression(env, root_and_current_tables, expr))
            .try_fold(
                sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
                |acc, expr| {
                    let right = expr?;
                    Ok(sql::ast::Expression::Or {
                        left: Box::new(acc),
                        right: Box::new(right),
                    })
                },
            ),
        models::Expression::Not { expression } => {
            let expr = translate_expression(env, root_and_current_tables, *expression)?;
            Ok(sql::ast::Expression::Not(Box::new(expr)))
        }
        models::Expression::BinaryComparisonOperator {
            column,
            operator,
            value,
        } => {
            let left = translate_comparison_target(env, root_and_current_tables, *column)?;
            let right = translate_comparison_value(env, root_and_current_tables, *value)?;
            Ok(sql::ast::Expression::BinaryOperator {
                left: Box::new(left),
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
                right: Box::new(right),
            })
        }
        models::Expression::BinaryArrayComparisonOperator {
            column,
            operator,
            values,
        } => {
            let left = translate_comparison_target(env, root_and_current_tables, *column.clone())?;
            let right = values
                .iter()
                .map(|value| {
                    translate_comparison_value(env, root_and_current_tables, value.clone())
                })
                .collect::<Result<Vec<sql::ast::Expression>, Error>>()?;
            Ok(sql::ast::Expression::BinaryArrayOperator {
                left: Box::new(left),
                operator: match *operator {
                    models::BinaryArrayComparisonOperator::In => sql::ast::BinaryArrayOperator::In,
                },
                right,
            })
        }

        // dummy
        models::Expression::Exists {
            in_collection,
            predicate,
        } => {
            translate_exists_in_collection(env, root_and_current_tables, *in_collection, *predicate)
        }
        // dummy
        models::Expression::UnaryComparisonOperator { column, operator } => match *operator {
            models::UnaryComparisonOperator::IsNull => {
                let value = translate_comparison_target(env, root_and_current_tables, *column)?;

                Ok(sql::ast::Expression::UnaryOperator {
                    column: Box::new(value),
                    operator: sql::ast::UnaryOperator::IsNull,
                })
            }
        },
    }
}

/// translate a comparison target.
fn translate_comparison_target(
    env: &Env,
    root_and_current_tables: &RootAndCurrentTables,
    column: models::ComparisonTarget,
) -> Result<sql::ast::Expression, Error> {
    match column {
        models::ComparisonTarget::Column { name, .. } => {
            let RootAndCurrentTables { current_table, .. } = root_and_current_tables;
            // get the unrelated table information from the metadata.
            let table_info = env
                .metadata
                .tables
                .0
                .get(&current_table.name)
                .ok_or(Error::TableNotFound(current_table.name.clone()))?;

            let metadata::ColumnInfo { name, .. } =
                table_info
                    .columns
                    .get(&name)
                    .ok_or(Error::ColumnNotFoundInTable(
                        name.clone(),
                        current_table.name.clone(),
                    ))?;

            Ok(sql::ast::Expression::ColumnName(
                sql::ast::ColumnName::TableColumn {
                    table: sql::ast::TableName::AliasedTable(current_table.reference.clone()),
                    name: name.to_string(),
                },
            ))
        }

        // Compare a column from the root table.
        models::ComparisonTarget::RootCollectionColumn { name } => {
            let RootAndCurrentTables { root_table, .. } = root_and_current_tables;
            // get the unrelated table information from the metadata.
            let table_info = env
                .metadata
                .tables
                .0
                .get(&root_table.name)
                .ok_or(Error::TableNotFound(root_table.name.to_string()))?;

            // find the requested column in the tables columns.
            let metadata::ColumnInfo { name, .. } =
                table_info
                    .columns
                    .get(&name)
                    .ok_or(Error::ColumnNotFoundInTable(
                        name.clone(),
                        root_table.name.to_string(),
                    ))?;

            Ok(sql::ast::Expression::ColumnName(
                sql::ast::ColumnName::TableColumn {
                    table: sql::ast::TableName::AliasedTable(root_table.reference.clone()),
                    name: name.to_string(),
                },
            ))
        }
    }
}

/// translate a comparison value.
fn translate_comparison_value(
    env: &Env,
    root_and_current_tables: &RootAndCurrentTables,
    value: models::ComparisonValue,
) -> Result<sql::ast::Expression, Error> {
    match value {
        models::ComparisonValue::Column { column } => {
            translate_comparison_target(env, root_and_current_tables, *column)
        }
        models::ComparisonValue::Scalar { value: json_value } => Ok(sql::ast::Expression::Value(
            translate_json_value(&json_value),
        )),
        models::ComparisonValue::Variable { name: var } => {
            Ok(sql::ast::Expression::Value(sql::ast::Value::Variable(var)))
        }
    }
}

/// Convert a JSON value into a SQL value.
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

/// Translate an EXISTS clause into a SQL subquery of the following form:
///
/// > EXISTS (SELECT FROM <table> AS <alias> WHERE <predicate>)
pub fn translate_exists_in_collection(
    env: &Env,
    root_and_current_tables: &RootAndCurrentTables,
    in_collection: models::ExistsInCollection,
    predicate: models::Expression,
) -> Result<sql::ast::Expression, Error> {
    match in_collection {
        // ignore arguments for now
        models::ExistsInCollection::Unrelated { collection, .. } => {
            // get the unrelated table information from the metadata.
            let table_info = env
                .metadata
                .tables
                .0
                .get(&collection)
                .ok_or(Error::TableNotFound(collection.clone()))?;

            // db table name
            let db_table_name: sql::ast::TableName = sql::ast::TableName::DBTable {
                schema: table_info.schema_name.clone(),
                table: table_info.table_name.clone(),
            };

            // new alias for the table
            let table_alias: sql::ast::TableAlias =
                sql::helpers::make_table_alias(collection.clone());

            // build a SELECT querying this table with the relevant predicate.
            let mut select = simple_select(vec![]);
            select.from = Some(sql::ast::From::Table {
                name: db_table_name.clone(),
                alias: table_alias.clone(),
            });

            let new_root_and_current_tables = RootAndCurrentTables {
                root_table: root_and_current_tables.root_table.clone(),
                current_table: TableNameAndReference {
                    reference: table_alias.clone(),
                    name: collection.clone(),
                },
            };

            let expr = translate_expression(env, &new_root_and_current_tables, predicate)?;
            select.where_ = sql::ast::Where(expr);

            // > EXISTS (SELECT FROM <table> AS <alias> WHERE <predicate>)
            Ok(sql::ast::Expression::Exists {
                select: Box::new(select),
            })
        }
        // We get a relationship name in exists, query the target table directly,
        // and build a WHERE clause that contains the join conditions and the specified
        // EXISTS condition.
        models::ExistsInCollection::Related { relationship, .. } => {
            // get the relationship table
            let relationship = env
                .relationships
                .get(&relationship)
                .ok_or(Error::RelationshipNotFound(relationship.clone()))?;

            // I don't expect v3-engine to let us down, but just in case :)
            if root_and_current_tables.current_table.name != relationship.source_collection_or_type
            {
                Err(Error::TableNotFound(
                    relationship.source_collection_or_type.clone(),
                ))
            } else {
                Ok(())
            }?;

            // get the unrelated table information from the metadata.
            let table_info = env
                .metadata
                .tables
                .0
                .get(&relationship.target_collection)
                .ok_or(Error::TableNotFound(relationship.target_collection.clone()))?;

            // relationship target db table name
            let db_table_name: sql::ast::TableName = sql::ast::TableName::DBTable {
                schema: table_info.schema_name.clone(),
                table: table_info.table_name.clone(),
            };

            // new alias for the target table
            let table_alias: sql::ast::TableAlias =
                sql::helpers::make_table_alias(relationship.target_collection.clone());
            let table_alias_name: sql::ast::TableName =
                sql::ast::TableName::AliasedTable(table_alias.clone());

            // build a SELECT querying this table with the relevant predicate.
            let mut select = simple_select(vec![]);
            select.from = Some(sql::ast::From::Table {
                name: db_table_name.clone(),
                alias: table_alias.clone(),
            });

            let new_root_and_current_tables = RootAndCurrentTables {
                root_table: root_and_current_tables.root_table.clone(),
                current_table: TableNameAndReference {
                    reference: table_alias.clone(),
                    name: relationship.target_collection.clone(),
                },
            };

            // exists condition
            let exists_cond = translate_expression(env, &new_root_and_current_tables, predicate)?;

            // relationship where clause
            let cond = relationships::translate_column_mapping(
                env,
                &root_and_current_tables.current_table,
                &table_alias_name,
                exists_cond,
                relationship,
            )?;

            select.where_ = sql::ast::Where(cond);

            // > EXISTS (SELECT FROM <table> AS <alias> WHERE <predicate>)
            Ok(sql::ast::Expression::Exists {
                select: Box::new(select),
            })
        }
    }
}
