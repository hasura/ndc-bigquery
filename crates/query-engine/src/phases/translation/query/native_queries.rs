//! Handle native queries translation after building the query.

use super::helpers::State;
use crate::phases::translation::sql;

/// Translate native queries collected in State by the translation proccess into CTEs.
pub fn translate(state: State) -> Vec<sql::ast::CommonTableExpression> {
    let mut ctes = vec![];
    let native_queries = state.get_native_queries();

    for (info, table_name) in native_queries {
        ctes.push(sql::ast::CommonTableExpression {
            table_name,
            column_names: None,
            select: sql::ast::CTExpr::Raw(sql::ast::Raw(info.sql)),
        });
    }

    ctes
}
