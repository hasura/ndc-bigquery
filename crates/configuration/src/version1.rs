//! Internal Configuration and state for our connector.

use crate::environment::Environment;
use crate::error::WriteParsedConfigurationError;
use crate::values::{self, ConnectionUri, PoolSettings, Secret};

use super::error::ParseConfigurationError;
use gcp_bigquery_client::model::job_configuration_query::JobConfigurationQuery;
use gcp_bigquery_client::model::query_request::QueryRequest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnection;
use sqlx::{Connection, Executor, Row};
use tracing::{info_span, Instrument};
use std::borrow::Cow;
use std::collections::BTreeMap;
use tokio::fs;
use std::path::Path;
use thiserror::Error;

//TODO(PY): temp, needs to be removed from the crate
// use ndc_sdk::connector;

use query_engine_metadata::metadata;

const CURRENT_VERSION: u32 = 1;
pub const CONFIGURATION_FILENAME: &str = "configuration.json";
pub const DEFAULT_CONNECTION_URI_VARIABLE: &str = "CONNECTION_URI";
const CONFIGURATION_QUERY: &str = include_str!("configuration.sql");
const CONFIGURATION_JSONSCHEMA_FILENAME: &str = "schema.json";

/// Initial configuration, just enough to connect to a database and elaborate a full
/// 'Configuration'.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct ParsedConfiguration {
    // Which version of the configuration format are we using
    pub version: u32,
    // Connection string for a Postgres-compatible database
    pub connection_uri: ConnectionUri,
    #[serde(skip_serializing_if = "PoolSettings::is_default")]
    #[serde(default)]
    pub pool_settings: PoolSettings,
    #[serde(default)]
    pub metadata: metadata::Metadata,
    // #[serde(default)]
    // pub aggregate_functions: metadata::AggregateFunctions,
}

impl ParsedConfiguration {
    pub fn initial() -> Self {
        ParsedConfiguration::empty()
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize, JsonSchema)]
pub enum Version {
    #[serde(rename = "1")]
    This,
}

/// User configuration, elaborated from a 'RawConfiguration'.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct Configuration {
    pub config: ParsedConfiguration,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub write_regions: Vec<RegionName>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub read_regions: Vec<RegionName>,
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    /// Routing table which relates the regions the NDC may be deployed in with the regions that
    /// the database is deployed, in order of preference.
    pub region_routing: BTreeMap<HasuraRegionName, Vec<RegionName>>,
}

/// Type that accept both a single value and a list of values. Allows for a simpler format when a
/// single value is the common case.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum SingleOrList<T> {
    Single(T),
    List(Vec<T>),
}

impl<T: Clone> SingleOrList<T> {
    fn is_empty(&self) -> bool {
        match self {
            SingleOrList::Single(_) => false,
            SingleOrList::List(l) => l.is_empty(),
        }
    }

    fn to_vec(&self) -> Vec<T> {
        match self {
            SingleOrList::Single(s) => vec![s.clone()],
            SingleOrList::List(l) => l.clone(),
        }
    }
}

// #[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
// #[serde(untagged)]
// pub enum ConnectionUris {
//     SingleRegion(SingleOrList<String>),
//     MultiRegion(MultipleRegionsConnectionUris),
// }

// pub fn single_connection_uri(connection_uri: String) -> ConnectionUris {
//     ConnectionUris::SingleRegion(SingleOrList::Single(connection_uri))
// }

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct MultipleRegionsConnectionUris {
    pub writes: BTreeMap<RegionName, SingleOrList<String>>,
    pub reads: BTreeMap<RegionName, SingleOrList<String>>,
}

/// Name of a region that the ndc may be deployed into.
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Ord, Deserialize, Serialize, JsonSchema)]
pub struct HasuraRegionName(pub String);

impl std::fmt::Display for HasuraRegionName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let HasuraRegionName(region) = self;
        write!(f, "{}", region)
    }
}

/// Name of a region that database servers may live in. These regions are distinct from the regions
/// the ndc can live in, and they need not be related a priori.
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Ord, Deserialize, Serialize, JsonSchema)]
pub struct RegionName(pub String);

impl std::fmt::Display for RegionName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let RegionName(region) = self;
        write!(f, "{}", region)
    }
}

impl ParsedConfiguration {
    pub fn empty() -> Self {
        Self {
            version: CURRENT_VERSION,
            connection_uri: ConnectionUri(Secret::FromEnvironment {
                 variable: DEFAULT_CONNECTION_URI_VARIABLE.into(),
            }),
            pool_settings: PoolSettings::default(),
            metadata: metadata::Metadata::default(),
            // aggregate_functions: metadata::AggregateFunctions::default(),
        }
    }
}

// /// Settings for the PostgreSQL connection pool
// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
// pub struct PoolSettings {
//     /// maximum number of pool connections
//     #[serde(default = "max_connection_default")]
//     pub max_connections: u32,
//     /// timeout for acquiring a connection from the pool (seconds)
//     #[serde(default = "pool_timeout_default")]
//     pub pool_timeout: u64,
//     /// idle timeout for releasing a connection from the pool (seconds)
//     #[serde(default = "idle_timeout_default")]
//     pub idle_timeout: Option<u64>,
//     /// maximum lifetime for an individual connection (seconds)
//     #[serde(default = "connection_lifetime_default")]
//     pub connection_lifetime: Option<u64>,
// }

// impl PoolSettings {
//     fn is_default(&self) -> bool {
//         *self == PoolSettings::default()
//     }
// }

// /// <https://hasura.io/docs/latest/api-reference/syntax-defs/#pgpoolsettings>
// impl Default for PoolSettings {
//     fn default() -> PoolSettings {
//         PoolSettings {
//             max_connections: 50,
//             pool_timeout: 600,
//             idle_timeout: Some(180),
//             connection_lifetime: Some(600),
//         }
//     }
// }

// // for serde default //
// fn max_connection_default() -> u32 {
//     PoolSettings::default().max_connections
// }
// fn pool_timeout_default() -> u64 {
//     PoolSettings::default().pool_timeout
// }
// fn idle_timeout_default() -> Option<u64> {
//     PoolSettings::default().idle_timeout
// }
// fn connection_lifetime_default() -> Option<u64> {
//     PoolSettings::default().connection_lifetime
// }

// TODO(PY): fix validate_raw_configuration----------------
/// Validate the user configuration.
// pub async fn validate_raw_configuration(
//     rawconfiguration: &ParsedConfiguration,
// ) -> Result<Configuration, ParseConfigurationError> {
//     if rawconfiguration.version != 1 {
//         return Err(connector::ValidateError::ValidateError(vec![
//             connector::InvalidRange {
//                 path: vec![connector::KeyOrIndex::Key("version".into())],
//                 message: format!(
//                     "invalid configuration version, expected 1, got {0}",
//                     rawconfiguration.version
//                 ),
//             },
//         ]));
//     }

//     match &rawconfiguration.connection_uris {
//         ConnectionUris::SingleRegion(urls) if urls.is_empty() => {
//             Err(connector::ParseError::ValidateError(vec![
//                 connector::InvalidRange {
//                     path: vec![connector::KeyOrIndex::Key("connection_uris".into())],
//                     message: "At least one database url must be specified".to_string(),
//                 },
//             ]))
//         }
//         ConnectionUris::MultiRegion(MultipleRegionsConnectionUris { reads, writes }) => {
//             let reads_empty_err = if reads.is_empty() {
//                 vec![connector::InvalidRange {
//                     path: vec![
//                         connector::KeyOrIndex::Key("connection_uris".into()),
//                         connector::KeyOrIndex::Key("reads".into()),
//                     ],
//                     message: "At least one 'reads' region must be specified".to_string(),
//                 }]
//             } else {
//                 vec![]
//             };
//             let reads_errs = reads
//                 .iter()
//                 .flat_map(|(RegionName(region), urls)| {
//                     if urls.is_empty() {
//                         vec![connector::InvalidRange {
//                             path: vec![
//                                 connector::KeyOrIndex::Key("connection_uris".into()),
//                                 connector::KeyOrIndex::Key("reads".into()),
//                                 connector::KeyOrIndex::Key(region.into()),
//                             ],
//                             message: "At least one database url must be specified".to_string(),
//                         }]
//                     } else {
//                         vec![]
//                     }
//                 })
//                 .collect::<Vec<connector::InvalidRange>>();
//             let writes_errs = writes
//                 .iter()
//                 .flat_map(|(RegionName(region), urls)| {
//                     if urls.is_empty() {
//                         vec![connector::InvalidRange {
//                             path: vec![
//                                 connector::KeyOrIndex::Key("connection_uris".into()),
//                                 connector::KeyOrIndex::Key("writes".into()),
//                                 connector::KeyOrIndex::Key(region.into()),
//                             ],
//                             message: "At least one database url must be specified".to_string(),
//                         }]
//                     } else {
//                         vec![]
//                     }
//                 })
//                 .collect::<Vec<connector::InvalidRange>>();

//             let mut errs = vec![];

//             errs.extend(reads_empty_err);
//             errs.extend(reads_errs);
//             errs.extend(writes_errs);

//             if !errs.is_empty() {
//                 Err(connector::ValidateError::ValidateError(errs))
//             } else {
//                 Ok(())
//             }
//         }
//         _ => Ok(()),
//     }?;

//     // Collect the regions that have been specified, to enable geo-localised deployments.
//     let (write_regions, read_regions) = match &rawconfiguration.connection_uris {
//         ConnectionUris::MultiRegion(MultipleRegionsConnectionUris { writes, reads }) => (
//             writes.keys().cloned().collect::<Vec<_>>(),
//             reads.keys().cloned().collect::<Vec<_>>(),
//         ),
//         ConnectionUris::SingleRegion(_) => (vec![], vec![]),
//     };

//     // region routing is provided by the metadata build service before the
//     // agent is deployed, so we don't need to try and calculate it here.
//     let region_routing = BTreeMap::new();

//     Ok(Configuration {
//         config: rawconfiguration.clone(),
//         write_regions,
//         read_regions,
//         region_routing,
//     })
// }

// /// Select the first available connection uri. Suitable for when hasura regions are not yet mapped
// /// to application regions.
// pub fn select_first_connection_url(urls: &ConnectionUris) -> String {
//     match &urls {
//         ConnectionUris::SingleRegion(urls) => urls.to_vec()[0].clone(),
//         ConnectionUris::MultiRegion(MultipleRegionsConnectionUris { reads, .. }) => reads
//             .first_key_value()
//             .expect("No regions are defined (Guarded by validate_raw_configuration)")
//             .1
//             .to_vec()[0]
//             .clone(),
//     }
// }

/// Construct the deployment configuration by introspecting the database.
pub async fn configure(
    args: &ParsedConfiguration,
    environment: impl Environment,
    // configuration_query: &str,
) -> anyhow::Result<ParsedConfiguration> {
    // dbg!(args);
    let uri = match &args.connection_uri {
        ConnectionUri(Secret::Plain(value)) => Cow::Borrowed(value),
        ConnectionUri(Secret::FromEnvironment { variable }) => {
            Cow::Owned(environment.read(variable)?)
        }
    };

    // .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;
    
    // let service_account_key_json = std::env::var("HASURA_BIGQUERY_SERVICE_KEY").unwrap();
    // dbg!(uri.as_ref().as_str());
    
    let service_account_key = yup_oauth2::parse_service_account_key(uri.as_str()).unwrap();
    
    // Init BigQuery client
    let bigquery_client =
        gcp_bigquery_client::Client::from_service_account_key(service_account_key, false)
            .await
            .unwrap();
    let query_request = QueryRequest::new(CONFIGURATION_QUERY);
    let project_id = "hasura-development";

    let datasets = bigquery_client
        .dataset()
        .list(project_id, Default::default())
        .await
        .unwrap();
    dbg!(datasets);

    let dataset_id = "chinook_sample";
    
    let rs = bigquery_client
        .job()
        // .query_all_with_location
        //     (project_id, 
        //         "EU", 
        //         JobConfigurationQuery {
        //             query: CONFIGURATION_QUERY,
        //             query_parameters: None,
        //             use_legacy_sql: Some(false),
        //             ..Default::default()
        //         }, 
        //         Some(2)
        //     );
        // .collect::<Result<Vec<_>, _>>()
        .query(project_id, query_request)
        .await
        // .map(|vec_of_vecs| vec_of_vecs.into_iter().flatten().collect());
        .unwrap();
        // ;

    dbg!(rs);
    // let r = rs.query_response().rows.unwrap().get(0).unwrap();
    // dbg!(r);

    let mut connection = PgConnection::connect(uri.as_str())
        .await?;

    let row = connection // TODO(PY): why is this PG connection
        .fetch_one(CONFIGURATION_QUERY)
        .await?;
        // .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    // let (scalar_types, composite_types) = transitively_occurring_types(
    //     occurring_scalar_types(
    //         &tables,
    //         &args.metadata.native_queries,
    //         &args.metadata.aggregate_functions,
    //     ),
    //     &occurring_composite_types(&tables, &args.metadata.native_queries),
    //     composite_types,
    // );
    let (scalar_types, composite_types) = async {
        let scalar_types: metadata::ScalarTypes = serde_json::from_value(row.get(1))?;
        let composite_types: metadata::CompositeTypes = serde_json::from_value(row.get(2))?;

        // We need to specify the concrete return type explicitly so that rustc knows that it can
        // be sent across an async boundary.
        // (last verified with rustc 1.72.1)
        Ok::<_, anyhow::Error>((scalar_types, composite_types))
    }
    .instrument(info_span!("Decode introspection result"))
    .await?;

    let tables: metadata::TablesInfo = serde_json::from_value(row.get(0))?;
        // .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    // let aggregate_functions: metadata::AggregateFunctions = serde_json::from_value(row.get(1))?;
        // .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    // let comparison_operators: metadata::ComparisonOperators = serde_json::from_value(row.get(2))
    //     .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    Ok(ParsedConfiguration {
        version: 1,
        connection_uri: args.connection_uri.clone(),
        pool_settings: args.pool_settings.clone(),
        metadata: metadata::Metadata {
            tables,
            native_operations: args.metadata.native_operations.clone(),
            scalar_types: scalar_types,
            composite_types: composite_types,
        },
        // aggregate_functions,
    })
}

/// Parse the configuration format from a directory.
pub async fn parse_configuration(
    configuration_dir: impl AsRef<Path>,
) -> Result<ParsedConfiguration, ParseConfigurationError> {
    let configuration_file = configuration_dir.as_ref().join(CONFIGURATION_FILENAME);

    let configuration_file_contents =
        fs::read_to_string(&configuration_file)
            .await
            .map_err(|err| {
                ParseConfigurationError::IoErrorButStringified(format!(
                    "{}: {}",
                    &configuration_file.display(),
                    err
                ))
            })?;

    let mut parsed_config: ParsedConfiguration = serde_json::from_str(&configuration_file_contents)
        .map_err(|error| ParseConfigurationError::ParseError {
            file_path: configuration_file.clone(),
            line: error.line(),
            column: error.column(),
            message: error.to_string(),
        })?;

    // look for native query sql file references and read from disk.
    for native_query_sql in parsed_config
        .metadata
        .native_operations
        .queries
        .0
        .values_mut()
    {
        native_query_sql.sql = metadata::NativeQuerySqlEither::NativeQuerySql(
            native_query_sql
                .sql
                .from_external(configuration_dir.as_ref())
                .map_err(ParseConfigurationError::IoErrorButStringified)?,
        );
    }
    for native_query_sql in parsed_config
        .metadata
        .native_operations
        .mutations
        .0
        .values_mut()
    {
        native_query_sql.sql = metadata::NativeQuerySqlEither::NativeQuerySql(
            native_query_sql
                .sql
                .from_external(configuration_dir.as_ref())
                .map_err(ParseConfigurationError::IoErrorButStringified)?,
        );
    }

    Ok(parsed_config)
}

/// Write the parsed configuration into a directory on disk.
pub async fn write_parsed_configuration(
    parsed_config: ParsedConfiguration,
    out_dir: impl AsRef<Path>,
) -> Result<(), WriteParsedConfigurationError> {
    let configuration_file = out_dir.as_ref().to_owned().join(CONFIGURATION_FILENAME);
    fs::create_dir_all(out_dir.as_ref()).await?;

    // create the configuration file
    fs::write(
        configuration_file,
        serde_json::to_string_pretty(&parsed_config)
            .map_err(|e| WriteParsedConfigurationError::IoError(e.into()))?
            + "\n",
    )
    .await?;

    // look for native query sql file references and write them to disk.
    for native_query_sql in parsed_config.metadata.native_operations.queries.0.values() {
        if let metadata::NativeQuerySqlEither::NativeQuerySql(
            metadata::NativeQuerySql::FromFile { file, sql },
        ) = &native_query_sql.sql
        {
            if file.is_absolute() || file.starts_with("..") {
                Err(
                    WriteParsedConfigurationError::WritingOutsideDestinationDir {
                        dir: out_dir.as_ref().to_owned(),
                        file: file.clone(),
                    },
                )?;
            };

            let native_query_file = out_dir.as_ref().to_owned().join(file);
            if let Some(native_query_sql_dir) = native_query_file.parent() {
                fs::create_dir_all(native_query_sql_dir).await?;
            };
            fs::write(native_query_file, String::from(sql.clone())).await?;
        };
    }
    for native_query_sql in parsed_config
        .metadata
        .native_operations
        .mutations
        .0
        .values()
    {
        if let metadata::NativeQuerySqlEither::NativeQuerySql(
            metadata::NativeQuerySql::FromFile { file, sql },
        ) = &native_query_sql.sql
        {
            if file.is_absolute() || file.starts_with("..") {
                Err(
                    WriteParsedConfigurationError::WritingOutsideDestinationDir {
                        dir: out_dir.as_ref().to_owned(),
                        file: file.clone(),
                    },
                )?;
            };

            let native_query_file = out_dir.as_ref().to_owned().join(file);
            if let Some(native_query_sql_dir) = native_query_file.parent() {
                fs::create_dir_all(native_query_sql_dir).await?;
            };
            fs::write(native_query_file, String::from(sql.clone())).await?;
        };
    }

    // create the jsonschema file
    let configuration_jsonschema_file_path = out_dir
        .as_ref()
        .to_owned()
        .join(CONFIGURATION_JSONSCHEMA_FILENAME);

    let output = schemars::schema_for!(ParsedConfiguration);
    fs::write(
        &configuration_jsonschema_file_path,
        serde_json::to_string_pretty(&output)
            .map_err(|e| WriteParsedConfigurationError::IoError(e.into()))?
            + "\n",
    )
    .await?;

    Ok(())
}

/// Configuration interpretation errors.
#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("error mapping hasura region to application region: {0}")]
    UnableToMapHasuraRegion(HasuraRegionName),
    #[error("error mapping application region to connection uris: {0}")]
    UnableToMapApplicationRegion(RegionName),
    #[error("DDN_REGION is not set, but is required for multi-region configuration")]
    DdnRegionIsNotSet,
}

// /// Filter predicate for comparison operators. Preserves only comparison operators that are
// /// relevant to any of the given scalar types.
// ///
// /// This function is public to enable use in later versions that retain the same metadata types.
// fn filter_comparison_operators(
//     scalar_types: &BTreeSet<models::ScalarTypeName>,
//     comparison_operators: metadata::ComparisonOperators,
// ) -> metadata::ComparisonOperators {
//     metadata::ComparisonOperators(
//         comparison_operators
//             .0
//             .into_iter()
//             .filter(|(typ, _)| scalar_types.contains(typ))
//             .map(|(typ, ops)| {
//                 (
//                     typ,
//                     ops.into_iter()
//                         .filter(|(_, op)| scalar_types.contains(&op.argument_type))
//                         .collect(),
//                 )
//             })
//             .collect(),
//     )
// }

// /// Filter predicate for aggregation functions. Preserves only aggregation functions that are
// /// relevant to any of the given scalar types.
// ///
// /// This function is public to enable use in later versions that retain the same metadata types.
// fn filter_aggregate_functions(
//     scalar_types: &BTreeSet<models::ScalarTypeName>,
//     aggregate_functions: metadata::AggregateFunctions,
// ) -> metadata::AggregateFunctions {
//     metadata::AggregateFunctions(
//         aggregate_functions
//             .0
//             .into_iter()
//             .filter(|(typ, _)| scalar_types.contains(typ))
//             .collect(),
//     )
// }
