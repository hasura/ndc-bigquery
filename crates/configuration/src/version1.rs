//! Internal Configuration and state for our connector.

use crate::environment::Environment;
use crate::error::WriteParsedConfigurationError;
use crate::values::{ConnectionUri, DatasetId, PoolSettings, ProjectId, Secret};

use super::error::ParseConfigurationError;
use gcp_bigquery_client::model::query_request::QueryRequest;
use ndc_models::{AggregateFunctionName, ComparisonOperatorName, ScalarTypeName, TypeName};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;
use tokio::fs;

//TODO(PY): temp, needs to be removed from the crate
// use ndc_sdk::connector;

use query_engine_metadata::metadata::{self, database, TablesInfo};

const CURRENT_VERSION: u32 = 1;
pub const CONFIGURATION_FILENAME: &str = "configuration.json";
pub const DEFAULT_SERVICE_KEY_VARIABLE: &str = "HASURA_BIGQUERY_SERVICE_KEY";
pub const DEFAULT_PROJECT_ID_VARIABLE: &str = "HASURA_BIGQUERY_PROJECT_ID";
pub const DEFAULT_DATASET_ID_VARIABLE: &str = "HASURA_BIGQUERY_DATASET_ID";
const CONFIGURATION_QUERY: &str = include_str!("config2.sql");
const CONFIGURATION_JSONSCHEMA_FILENAME: &str = "schema.json";

const CHARACTER_STRINGS: [&str; 3] = ["character", "text", "string"];
const UNICODE_CHARACTER_STRINGS: [&str; 3] = ["nchar", "ntext", "nvarchar"];
const CANNOT_COMPARE: [&str; 3] = ["text", "ntext", "image"];
const EXACT_NUMERICS: [&str; 9] = [
    "bigint",
    "bit",
    "decimal",
    "int",
    "money",
    "numeric",
    "smallint",
    "smallmoney",
    "tinyint",
];
const APPROX_NUMERICS: [&str; 3] = ["float", "real", "float64"];
const NOT_COUNTABLE: [&str; 3] = ["image", "ntext", "text"];
const NOT_APPROX_COUNTABLE: [&str; 4] = ["image", "sql_variant", "ntext", "text"];

/// Initial configuration, just enough to connect to a database and elaborate a full
/// 'Configuration'.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct ParsedConfiguration {
    // Which version of the configuration format are we using
    pub version: u32,
    // Connection string for a Postgres-compatible database
    pub service_key: ConnectionUri,
    pub project_id: ProjectId,
    pub dataset_id: DatasetId,
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
        write!(f, "{region}")
    }
}

/// Name of a region that database servers may live in. These regions are distinct from the regions
/// the ndc can live in, and they need not be related a priori.
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Ord, Deserialize, Serialize, JsonSchema)]
pub struct RegionName(pub String);

impl std::fmt::Display for RegionName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let RegionName(region) = self;
        write!(f, "{region}")
    }
}

impl ParsedConfiguration {
    pub fn empty() -> Self {
        Self {
            version: CURRENT_VERSION,
            service_key: ConnectionUri(Secret::FromEnvironment {
                variable: DEFAULT_SERVICE_KEY_VARIABLE.into(),
            }),
            project_id: ProjectId(Secret::FromEnvironment {
                variable: DEFAULT_PROJECT_ID_VARIABLE.into(),
            }),
            dataset_id: DatasetId(Secret::FromEnvironment {
                variable: DEFAULT_DATASET_ID_VARIABLE.into(),
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
) -> anyhow::Result<ParsedConfiguration> {
    let service_key = match &args.service_key {
        ConnectionUri(Secret::Plain(value)) => Cow::Borrowed(value),
        ConnectionUri(Secret::FromEnvironment { variable }) => {
            Cow::Owned(environment.read(variable)?)
        }
    };

    let project_id_ = match &args.project_id {
        ProjectId(Secret::Plain(value)) => Cow::Borrowed(value),
        ProjectId(Secret::FromEnvironment { variable }) => Cow::Owned(environment.read(variable)?),
    };

    let dataset_id_ = match &args.dataset_id {
        DatasetId(Secret::Plain(value)) => Cow::Borrowed(value),
        DatasetId(Secret::FromEnvironment { variable }) => Cow::Owned(environment.read(variable)?),
    };

    let service_account_key = yup_oauth2::parse_service_account_key(service_key.as_str()).unwrap();

    let project_id = project_id_.as_str();
    let dataset_id = dataset_id_.as_str();

    let schema_name = format!("{project_id}.{dataset_id}");
    let database_name = schema_name.clone();

    // Init BigQuery client
    let bigquery_client =
        gcp_bigquery_client::Client::from_service_account_key(service_account_key, false)
            .await
            .unwrap();

    // get scalar_types

    let types_query = format!(
        "select data_type from {project_id}.{dataset_id}.INFORMATION_SCHEMA.COLUMN_FIELD_PATHS"
    );

    let types_row = bigquery_client
        .job()
        .query(project_id, QueryRequest::new(types_query))
        .await
        .unwrap();

    let types_query_response = types_row.query_response().clone();

    //TODO(PY): too many unwraps!
    let types = types_query_response
        .rows
        .as_ref()
        .unwrap()
        .iter()
        .map(|row| TypeItem {
            name: serde_json::from_value(
                row.columns
                    .as_ref()
                    .unwrap()
                    .iter()
                    .next()
                    .unwrap()
                    .value
                    .as_ref()
                    .unwrap()
                    .to_owned(),
            )
            .unwrap(),
        })
        .collect::<Vec<_>>();

    let scalar_types = get_scalar_types(&types, schema_name);

    // get tables_info

    let config_query_string = CONFIGURATION_QUERY.to_string();

    let config_query_string_with_database_name: String =
        config_query_string.replace("hasura_database_name", database_name.as_str()); //TODO(PY): what is a safe name to provide as a variable name?

    let tables_query_request = QueryRequest::new(config_query_string_with_database_name);

    let tables_result = bigquery_client
        .job()
        .query(project_id, tables_query_request)
        .await
        .unwrap();

    let table_rows = tables_result.query_response().clone();

    let mut tables_info = TablesInfo::empty();

    for row in table_rows.rows.unwrap_or_default() {
        let configuration_table_info = if let Some(columns) = row.columns {
            if let Some(column) = columns.into_iter().next() {
                if let Some(value) = column.value {
                    if let serde_json::Value::String(str) = value {
                        if let Ok(table_info_map) = serde_json::from_str::<TablesInfo>(&str) {
                            // tables_info.merge(table_info_map);
                            Ok(table_info_map)
                        } else {
                            Err(format!("Failed to deserialize TablesInfo from JSON: {str}"))
                        }
                    } else {
                        Err(format!("Expected a string value, found: {value:?}"))
                    }
                } else {
                    Err("Missing value in columns".to_string())
                }
            } else {
                Err("Empty columns".to_string())
            }
        } else {
            Err("Empty rows".to_string())
        };
        if let Ok(table_info_map) = configuration_table_info {
            tables_info.merge(table_info_map);
        }
    }

    Ok(ParsedConfiguration {
        version: 1,
        service_key: args.service_key.clone(),
        project_id: args.project_id.clone(),
        dataset_id: args.dataset_id.clone(),
        pool_settings: args.pool_settings.clone(),
        metadata: metadata::Metadata {
            tables: tables_info,
            native_operations: args.metadata.native_operations.clone(),
            scalar_types,
            // composite_types: CompositeTypes::empty(),
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

#[derive(Deserialize, Debug)]
struct TypeItem {
    name: ScalarTypeName,
}

// we hard code these, essentially
// we look up available types in `sys.types` but hard code their behaviour by looking them up below
// taken from https://learn.microsoft.com/en-us/sql/t-sql/functions/aggregate-functions-transact-sql?view=sql-server-ver16
fn get_aggregate_functions_for_type(
    type_name: &ndc_models::ScalarTypeName,
) -> BTreeMap<AggregateFunctionName, database::AggregateFunction> {
    let mut aggregate_functions = BTreeMap::new();

    if !NOT_APPROX_COUNTABLE.contains(&type_name.as_str()) {
        aggregate_functions.insert(
            AggregateFunctionName::new("APPROX_COUNT_DISTINCT".into()),
            database::AggregateFunction {
                return_type: TypeName::new("bigint".to_string().into()),
            },
        );
    }

    if !NOT_COUNTABLE.contains(&type_name.as_str()) {
        aggregate_functions.insert(
            AggregateFunctionName::new("COUNT".into()),
            database::AggregateFunction {
                return_type: TypeName::new("bigint".to_string().into()),
            },
        );
    }

    if type_name.as_str() != "bit"
        && (EXACT_NUMERICS.contains(&type_name.as_str())
            || APPROX_NUMERICS.contains(&type_name.as_str())
            || CHARACTER_STRINGS.contains(&type_name.as_str())
            || type_name.as_str() == "date"
            || type_name.as_str() == "datetime"
            || type_name.as_str() == "uuid")
    {
        aggregate_functions.insert(
            AggregateFunctionName::new("MIN".into()),
            database::AggregateFunction {
                return_type: TypeName::new(type_name.as_str().to_string().into()),
            },
        );
        aggregate_functions.insert(
            AggregateFunctionName::new("MAX".into()),
            database::AggregateFunction {
                return_type: TypeName::new(type_name.as_str().to_string().into()),
            },
        );
    }

    // if type_name.as_str() != "bit"
    //     && (EXACT_NUMERICS.contains(&type_name.as_str())
    //         || APPROX_NUMERICS.contains(&type_name.as_str()))
    // {
    //     aggregate_functions.insert(
    //         AggregateFunctionName::new("STDEV".into()),
    //         database::AggregateFunction {
    //             return_type: TypeName::new("float".to_string().into()),
    //         },
    //     );
    //     aggregate_functions.insert(
    //         AggregateFunctionName::new("STDEVP".into()),
    //         database::AggregateFunction {
    //             return_type: TypeName::new("float".to_string().into()),
    //         },
    //     );
    //     aggregate_functions.insert(
    //         AggregateFunctionName::new("VAR".into()),
    //         database::AggregateFunction {
    //             return_type: TypeName::new("float".to_string().into()),
    //         },
    //     );
    //     aggregate_functions.insert(
    //         AggregateFunctionName::new("VARP".into()),
    //         database::AggregateFunction {
    //             return_type: TypeName::new("float".to_string().into()),
    //         },
    //     );
    // }

    if let Some(precise_return_type) = match type_name.as_str() {
        "tinyint" => Some("smallint"),
        "smallint" => Some("smallint"),
        "int" => Some("integer"),
        "bigint" => Some("bigint"),
        "decimal" => Some("decimal"),
        "money" => Some("money"),
        "smallmoney" => Some("money"),
        "float" => Some("float"),
        "real" => Some("float"),
        "int64" => Some("bigint"),
        "int32" => Some("integer"),
        "int16" => Some("smallint"),
        _ => None,
    } {
        aggregate_functions.insert(
            AggregateFunctionName::new("AVG".into()),
            database::AggregateFunction {
                return_type: TypeName::new(precise_return_type.to_string().into()),
            },
        );
        aggregate_functions.insert(
            AggregateFunctionName::new("SUM".into()),
            database::AggregateFunction {
                return_type: TypeName::new(precise_return_type.to_string().into()),
            },
        );
    };

    // aggregate_functions.insert(
    //     AggregateFunctionName::new("COUNT_BIG".into()),
    //     database::AggregateFunction {
    //         return_type: TypeName::new("bigint".to_string().into()),
    //     },
    // );

    aggregate_functions
}

// we lookup all types in sys.types, then use our hardcoded ideas about each one to attach
// comparison operators
fn get_scalar_types(type_names: &Vec<TypeItem>, schema_name: String) -> database::ScalarTypes {
    let mut scalar_types = BTreeMap::new();
    let schema = if schema_name.is_empty() {
        None
    } else {
        Some(schema_name)
    };

    for type_item in type_names {
        let type_name = match type_item.name.as_str().to_lowercase().as_str() {
            "boolean" => "boolean",
            "int" => "integer",
            "int16" => "smallint",
            "smallint" => "smallint",
            "int32" => "integer",
            "integer" => "integer",
            "int64" => "bigint",
            "bigint" => "bigint",
            "numeric" => "numeric",
            "float64" => "float",
            "float" => "float",
            "real" => "real",
            "double precision" => "double precision",
            "text" => "text",
            "string" => "string",
            "character" => "character",
            "json" => "json",
            "jsonb" => "jsonb",
            "date" => "date",
            "time with time zone" => "time with time zone",
            "time without time zone" => "time without time zone",
            "timestamp with time zone" => "timestamp with time zone",
            "timestamp without time zone" => "timestamp without time zone",
            "uuid" => "uuid",
            _ => "any",
        };
        let type_name_scalar = ScalarTypeName::new(type_name.into());
        scalar_types.insert(
            type_name_scalar.clone(),
            database::ScalarType {
                type_name: type_name_scalar.clone(),
                schema_name: schema.clone(),
                comparison_operators: get_comparison_operators_for_type(&type_name_scalar),
                aggregate_functions: get_aggregate_functions_for_type(&type_name_scalar),
                description: None,
                type_representation: None,
            },
            // get_comparison_operators_for_type(&type_name.name),
        );
    }

    database::ScalarTypes(scalar_types)
}

// we hard code these, essentially
// we look up available types in `sys.types` but hard code their behaviour by looking them up below
// categories taken from https://learn.microsoft.com/en-us/sql/t-sql/data-types/data-types-transact-sql
fn get_comparison_operators_for_type(
    type_name: &ndc_models::ScalarTypeName,
) -> BTreeMap<ComparisonOperatorName, database::ComparisonOperator> {
    let mut comparison_operators = BTreeMap::new();

    // in ndc-spec, all things can be `==`
    comparison_operators.insert(
        ComparisonOperatorName::new("_eq".into()),
        database::ComparisonOperator {
            operator_name: "=".to_string(),
            argument_type: type_name.clone(),
            operator_kind: database::OperatorKind::Equal,
            is_infix: true,
        },
    );

    comparison_operators.insert(
        ComparisonOperatorName::new("_in".into()),
        database::ComparisonOperator {
            operator_name: "IN".to_string(),
            argument_type: type_name.clone(),
            operator_kind: database::OperatorKind::In,
            is_infix: true,
        },
    );

    // include LIKE and NOT LIKE for string-ish types
    if CHARACTER_STRINGS.contains(&type_name.as_str())
        || UNICODE_CHARACTER_STRINGS.contains(&type_name.as_str())
    {
        comparison_operators.insert(
            ComparisonOperatorName::new("_like".into()),
            database::ComparisonOperator {
                operator_name: "LIKE".to_string(),
                argument_type: type_name.clone(),
                operator_kind: database::OperatorKind::Custom,
                is_infix: true,
            },
        );
        comparison_operators.insert(
            ComparisonOperatorName::new("_nlike".into()),
            database::ComparisonOperator {
                operator_name: "NOT LIKE".to_string(),
                argument_type: type_name.clone(),
                operator_kind: database::OperatorKind::Custom,
                is_infix: true,
            },
        );
    }

    // include comparison operators for types that are comparable, according to
    // https://learn.microsoft.com/en-us/sql/t-sql/language-elements/comparison-operators-transact-sql?view=sql-server-ver16
    if !CANNOT_COMPARE.contains(&type_name.as_str()) {
        comparison_operators.insert(
            ComparisonOperatorName::new("_neq".into()),
            database::ComparisonOperator {
                operator_name: "!=".to_string(),
                argument_type: type_name.clone(),
                operator_kind: database::OperatorKind::Custom,
                is_infix: true,
            },
        );
        comparison_operators.insert(
            ComparisonOperatorName::new("_lt".into()),
            database::ComparisonOperator {
                operator_name: "<".to_string(),
                argument_type: type_name.clone(),
                operator_kind: database::OperatorKind::Custom,
                is_infix: true,
            },
        );
        comparison_operators.insert(
            ComparisonOperatorName::new("_gt".into()),
            database::ComparisonOperator {
                operator_name: ">".to_string(),
                argument_type: type_name.clone(),
                operator_kind: database::OperatorKind::Custom,
                is_infix: true,
            },
        );

        comparison_operators.insert(
            ComparisonOperatorName::new("_gte".into()),
            database::ComparisonOperator {
                operator_name: ">=".to_string(),
                argument_type: type_name.clone(),
                operator_kind: database::OperatorKind::Custom,
                is_infix: true,
            },
        );
        comparison_operators.insert(
            ComparisonOperatorName::new("_lte".into()),
            database::ComparisonOperator {
                operator_name: "<=".to_string(),
                argument_type: type_name.clone(),
                operator_kind: database::OperatorKind::Custom,
                is_infix: true,
            },
        );
    }
    comparison_operators
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
