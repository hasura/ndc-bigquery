//! Handle aggregates translation.

use super::error::Error;
use super::helpers;
use crate::phases::translation::sql;

use indexmap::IndexMap;
use ndc_client::models;

/// Translate any aggregates we should include in the query into our SQL AST.
pub fn translate(
    table: sql::ast::TableName,
    aggregates: IndexMap<String, models::Aggregate>,
) -> Result<Vec<(sql::ast::ColumnAlias, sql::ast::Expression)>, Error> {
    aggregates
        .into_iter()
        .map(|(alias, aggregation)| {
            let expression = match aggregation {
                models::Aggregate::ColumnCount { column, distinct } => {
                    let count_column_alias = helpers::make_column_alias(column);
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
                                name: helpers::make_column_alias(column),
                            },
                        )],
                    }
                }
                models::Aggregate::StarCount {} => {
                    sql::ast::Expression::Count(sql::ast::CountType::Star)
                }
            };
            Ok((helpers::make_column_alias(alias), expression))
        })
        .collect::<Result<Vec<_>, Error>>()
}
