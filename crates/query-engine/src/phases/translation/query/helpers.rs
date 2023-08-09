//! Helpers for processing the QueryRequest and building SQL.

use crate::phases::translation::sql;

/// For the root table in the query, and for the current table we are processing,
/// We'd like to track what is their reference in the query (the name we can use to address them,
/// an alias we generate), and what is their name in the metadata (so we can get
/// their information such as which columns are available for that table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootAndCurrentTables {
    /// The root (top-most) table in the query.
    pub root_table: TableNameAndReference,
    /// The current table we are processing.
    pub current_table: TableNameAndReference,
}

/// For a table in the query, We'd like to track what is its reference in the query
/// (the name we can use to address them, an alias we generate), and what is their name in the
/// metadata (so we can get their information such as which columns are available for that table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableNameAndReference {
    /// Table name for column lookup
    pub name: String,
    /// Table alias to query from
    pub reference: sql::ast::TableAlias,
}
