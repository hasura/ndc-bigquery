//! Metadata information regarding the database and tracked information.

pub mod database;
pub mod mutations;
pub mod native_queries;

// re-export without modules
pub use database::*;
pub use native_queries::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Metadata information.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct Metadata {
    pub tables: TablesInfo,
    // pub composite_types: CompositeTypes,
    pub native_operations: NativeOperations,
    pub scalar_types: ScalarTypes,
}

impl Metadata {
    pub fn empty() -> Self {
        Metadata {
            tables: TablesInfo::empty(),
            // composite_types: CompositeTypes::empty(),
            native_operations: NativeOperations::empty(),
            scalar_types: ScalarTypes::empty(),
        }
    }
}
