//! Helpers for processing the QueryRequest and building SQL.

use crate::metadata;
use crate::phases::translation::query::error::Error;
use crate::phases::translation::sql;
use ndc_hub::models;
use std::collections::BTreeMap;

/// Static information from the query and metadata.
pub struct Env {
    metadata: metadata::Metadata,
    pub relationships: BTreeMap<String, models::Relationship>,
}

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
    pub reference: sql::ast::TableName,
}

/// Metadata information about a specific collection.
pub enum CollectionInfo {
    Table {
        name: String,
        info: metadata::TableInfo,
    },
    NativeQuery {
        name: String,
        info: metadata::NativeQueryInfo,
    },
}

impl Env {
    /// Create a new Env by supplying the metadata and relationships.
    pub fn new(
        metadata: metadata::Metadata,
        relationships: BTreeMap<String, models::Relationship>,
    ) -> Env {
        Env {
            metadata,
            relationships,
        }
    }
    /// Lookup a collection's information in the metadata.
    pub fn lookup_collection(&self, collection_name: &str) -> Result<CollectionInfo, Error> {
        let table = self
            .metadata
            .tables
            .0
            .get(collection_name)
            .map(|t| CollectionInfo::Table {
                name: collection_name.to_string(),
                info: t.clone(),
            });

        match table {
            Some(table) => Ok(table),
            None => self
                .metadata
                .native_queries
                .0
                .get(collection_name)
                .map(|nq| CollectionInfo::NativeQuery {
                    name: collection_name.to_string(),
                    info: nq.clone(),
                })
                .ok_or(Error::CollectionNotFound(collection_name.to_string())),
        }
    }
}

impl CollectionInfo {
    /// Lookup a column in a collection.
    pub fn lookup_column(&self, column_name: &str) -> Result<&metadata::ColumnInfo, Error> {
        match self {
            CollectionInfo::Table { name, info } => {
                info.columns
                    .get(column_name)
                    .ok_or(Error::ColumnNotFoundInCollection(
                        column_name.to_string(),
                        name.clone(),
                    ))
            }
            CollectionInfo::NativeQuery { name, info } => {
                info.columns
                    .get(column_name)
                    .ok_or(Error::ColumnNotFoundInCollection(
                        column_name.to_string(),
                        name.clone(),
                    ))
            }
        }
    }
}
