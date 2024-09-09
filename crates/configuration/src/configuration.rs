//! Configuration for the connector.

use std::path::Path;

use query_engine_metadata::metadata;

use crate::environment::Environment;
use crate::error::{
    MakeRuntimeConfigurationError, MultiError, ParseConfigurationError,
    WriteParsedConfigurationError,
};
use crate::values::PoolSettings;
use schemars::{gen::SchemaSettings, schema::RootSchema};

/// The 'Configuration' type collects all the information necessary to serve queries at runtime.
///
/// 'ParsedConfiguration' deals with a multitude of different concrete version formats, and each
/// version is responsible for interpreting its serialized format into the current 'Configuration'.
/// Values of this type are produced from a 'ParsedConfiguration' using
/// 'make_runtime_configuration'.
///
/// Separating 'ParsedConfiguration' and 'Configuration' simplifies the main query translation
/// logic by placing the responsibility of dealing with configuration format evolution in
/// 'ParsedConfiguration.
///
#[derive(Debug)]
pub struct Configuration {
    pub metadata: metadata::Metadata,
    pub pool_settings: PoolSettings,
    pub connection_uri: String,
    // pub mutations_version: Option<metadata::mutations::MutationsVersion>,
}
