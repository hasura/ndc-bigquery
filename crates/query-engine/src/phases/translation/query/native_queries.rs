//! Handle native queries translation after building the query.

use ndc_sdk::models;

use super::error::Error;
use super::helpers::State;
use super::values;
use crate::metadata;
use crate::phases::translation::sql;

/// Translate native queries collected in State by the translation proccess into CTEs.
pub fn translate(state: State) -> Result<Vec<sql::ast::CommonTableExpression>, Error> {
    let mut ctes = vec![];
    let native_queries = state.get_native_queries();

    // for each found table expression
    for native_query in native_queries {
        // convert metadata representation to sql::ast representation
        let sql: Vec<sql::ast::RawSql> = native_query
            .info
            .sql
            .0
            .into_iter()
            .map(|part| match part {
                metadata::NativeQueryPart::Text(text) => Ok(sql::ast::RawSql::RawText(text)),
                metadata::NativeQueryPart::Parameter(param) => {
                    let value = match native_query.arguments.get(&param) {
                        None => Err(Error::ArgumentNotFound(param.clone())),
                        Some(argument) => match argument {
                            models::Argument::Literal { value } => {
                                values::translate_json_value(value)
                            }
                            models::Argument::Variable { name } => {
                                Ok(sql::ast::Value::Variable(name.clone()))
                            }
                        },
                    }?;
                    Ok(sql::ast::RawSql::Value(value))
                }
            })
            .collect::<Result<Vec<sql::ast::RawSql>, Error>>()?;

        // add a cte
        ctes.push(sql::ast::CommonTableExpression {
            alias: native_query.alias,
            column_names: None,
            select: sql::ast::CTExpr::RawSql(sql),
        });
    }

    Ok(ctes)
}
