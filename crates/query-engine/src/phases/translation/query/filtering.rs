//! Handle filtering/where clauses translation.

use super::error::Error;
use crate::metadata;
use crate::phases::translation::sql;

use ndc_client::models;

use std::collections::BTreeMap;

/// Translate a boolean expression to a SQL expression.
pub fn translate_expression(
    tables_info: &metadata::TablesInfo,
    relationships: &BTreeMap<String, models::Relationship>,
    // table alias to query from
    table: &sql::ast::TableName,
    // root table name for column lookup
    root_table_name: &String,
    predicate: models::Expression,
) -> Result<sql::ast::Expression, Error> {
    match predicate {
        models::Expression::And { expressions } => expressions
            .into_iter()
            .map(|expr| {
                translate_expression(tables_info, relationships, table, root_table_name, expr)
            })
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
            .map(|expr| {
                translate_expression(tables_info, relationships, table, root_table_name, expr)
            })
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
            let expr = translate_expression(
                tables_info,
                relationships,
                table,
                root_table_name,
                *expression,
            )?;
            Ok(sql::ast::Expression::Not(Box::new(expr)))
        }
        models::Expression::BinaryComparisonOperator {
            column,
            operator,
            value,
        } => {
            let left = translate_comparison_target(
                tables_info,
                relationships,
                root_table_name,
                table,
                *column,
            )?;
            let right = translate_comparison_value(
                tables_info,
                relationships,
                root_table_name,
                table,
                *value,
            )?;
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
            let left = translate_comparison_target(
                tables_info,
                relationships,
                root_table_name,
                table,
                *column.clone(),
            )?;
            let right = values
                .iter()
                .map(|value| {
                    translate_comparison_value(
                        tables_info,
                        relationships,
                        root_table_name,
                        table,
                        value.clone(),
                    )
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
        } => translate_exists_in_collection(tables_info, relationships, *in_collection, *predicate),
        // dummy
        models::Expression::UnaryComparisonOperator { column, operator } => match *operator {
            models::UnaryComparisonOperator::IsNull => {
                let value = translate_comparison_target(
                    tables_info,
                    relationships,
                    root_table_name,
                    table,
                    *column,
                )?;

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
    _tables_info: &metadata::TablesInfo,
    _relationships: &BTreeMap<String, models::Relationship>,
    _root_table_name: &str,
    table: &sql::ast::TableName,
    column: models::ComparisonTarget,
) -> Result<sql::ast::Expression, Error> {
    match column {
        models::ComparisonTarget::Column { name, .. } => Ok(sql::ast::Expression::ColumnName(
            sql::ast::ColumnName::TableColumn {
                table: table.clone(),
                name,
            },
        )),
        models::ComparisonTarget::RootCollectionColumn { .. } => todo!(),
    }
}

/// translate a comparison value.
fn translate_comparison_value(
    tables_info: &metadata::TablesInfo,
    relationships: &BTreeMap<String, models::Relationship>,
    root_table_name: &str,
    table: &sql::ast::TableName,
    value: models::ComparisonValue,
) -> Result<sql::ast::Expression, Error> {
    match value {
        models::ComparisonValue::Column { column } => {
            translate_comparison_target(tables_info, relationships, root_table_name, table, *column)
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
    tables_info: &metadata::TablesInfo,
    _relationships: &BTreeMap<String, models::Relationship>,
    in_collection: models::ExistsInCollection,
    _predicate: models::Expression,
) -> Result<sql::ast::Expression, Error> {
    match in_collection {
        // ignore arguments for now
        models::ExistsInCollection::Unrelated { collection, .. } => {
            // get the unrelated table information from the metadata.
            let metadata::TablesInfo(tables_info_map) = tables_info;
            let _table_info = tables_info_map
                .get(&collection)
                .ok_or(Error::TableNotFound(collection.clone()))?;
            // new alias for the table
            let _table_alias: sql::ast::TableAlias =
                sql::helpers::make_table_alias(collection.clone());

            todo!()
        }
        models::ExistsInCollection::Related { .. } => {
            todo!()
        }
    }
}
