//! helpers for query translation.

use crate::phases::translation::sql;

/// Generate a column expression refering to a specific table.
pub fn make_column(
    table: sql::ast::TableName,
    name: String,
    alias: sql::ast::ColumnAlias,
) -> (sql::ast::ColumnAlias, sql::ast::Expression) {
    (
        alias,
        sql::ast::Expression::ColumnName(sql::ast::ColumnName::TableColumn { table, name }),
    )
}

/// Create column aliases using this function so we build everything in one place.
/// We originally wanted indices, but we didn't end up using them.
/// Leaving them here for now, but will probably remove them in the future.
pub fn make_column_alias(name: String) -> sql::ast::ColumnAlias {
    sql::ast::ColumnAlias {
        unique_index: 0,
        name,
    }
}
/// Create table aliases using this function so they get a unique index.
/// We originally wanted indices, but we didn't end up using them.
/// Leaving them here for now, but will probably remove them in the future.
pub fn make_table_alias(name: String) -> sql::ast::TableAlias {
    sql::ast::TableAlias {
        unique_index: 0,
        name,
    }
}
