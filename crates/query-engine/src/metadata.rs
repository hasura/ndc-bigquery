/// Metadata information regarding the database and tracekd information.
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
/// Mapping from a graphql "table" name to its information.
pub struct TablesInfo(pub HashMap<String, TableInfo>);

#[derive(Debug, Deserialize)]
/// Information about a database table object.
pub struct TableInfo {
    pub schema_name: String,
    pub table_name: String,
    pub columns: HashMap<String, ColumnInfo>,
}

#[derive(Debug, Deserialize)]
/// Information about a database column object.
pub struct ColumnInfo {
    pub name: String,
}
