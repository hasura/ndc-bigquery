//! Handle filtering/where clauses translation.

use crate::phases::translation::sql;

use ndc_client::models;

pub fn translate_expression(
    table: &sql::ast::TableName,
    predicate: models::Expression,
) -> sql::ast::Expression {
    match predicate {
        models::Expression::And { expressions } => expressions
            .into_iter()
            .map(|expr| translate_expression(table, expr))
            .fold(
                sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
                |acc, expr| sql::ast::Expression::And {
                    left: Box::new(acc),
                    right: Box::new(expr),
                },
            ),
        models::Expression::Or { expressions } => expressions
            .into_iter()
            .map(|expr| translate_expression(table, expr))
            .fold(
                sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
                |acc, expr| sql::ast::Expression::Or {
                    left: Box::new(acc),
                    right: Box::new(expr),
                },
            ),
        models::Expression::Not { expression } => {
            sql::ast::Expression::Not(Box::new(translate_expression(table, *expression)))
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
