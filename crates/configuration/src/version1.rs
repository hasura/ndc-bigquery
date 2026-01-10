//! Internal Configuration and state for our connector.

use crate::connection_settings;
use crate::environment::Environment;
use crate::error::WriteParsedConfigurationError;
use crate::values::{DatasetId, PoolSettings, ProjectId, Secret, ServiceKey, WorkloadIdentityAuth};

use super::error::ParseConfigurationError;
use gcp_bigquery_client::model::query_request::QueryRequest;
use gcp_bigquery_client::model::table_cell::TableCell;
use gcp_bigquery_client::model::table_row::TableRow;
use ndc_models::{AggregateFunctionName, ComparisonOperatorName, ScalarTypeName, TypeName};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;
use tokio::fs;

//TODO(PY): temp, needs to be removed from the crate
// use ndc_sdk::connector;

use query_engine_metadata::metadata::{self, database, TablesInfo};

const CURRENT_VERSION: u32 = 1;
pub const CONFIGURATION_FILENAME: &str = "configuration.json";
pub const DEFAULT_SERVICE_KEY_VARIABLE: &str = "HASURA_BIGQUERY_SERVICE_KEY";
pub const DEFAULT_PROJECT_ID_VARIABLE: &str = "HASURA_BIGQUERY_PROJECT_ID";
pub const DEFAULT_DATASET_ID_VARIABLE: &str = "HASURA_BIGQUERY_DATASET_ID";
const CONFIGURATION_QUERY: &str = include_str!("configuration.sql");
const CONFIGURATION_JSONSCHEMA_FILENAME: &str = "schema.json";

/// Initial configuration, just enough to connect to a database and elaborate a full
/// 'Configuration'.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ParsedConfiguration {
    // Which version of the configuration format are we using
    pub version: u32,
    pub connection_settings: connection_settings::DatabaseConnectionSettings,
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

impl ParsedConfiguration {
    pub fn empty() -> Self {
        Self {
            version: CURRENT_VERSION,
            connection_settings: connection_settings::DatabaseConnectionSettings::empty(),
            pool_settings: PoolSettings::default(),
            metadata: metadata::Metadata::default(),
            // aggregate_functions: metadata::AggregateFunctions::default(),
        }
    }
}

/// Construct the deployment configuration by introspecting the database.
pub async fn configure(
    args: &ParsedConfiguration,
    environment: impl Environment,
) -> anyhow::Result<ParsedConfiguration> {
    let bigquery_client = match &args.connection_settings.workload_identity_auth {
        Some(WorkloadIdentityAuth(Secret::Plain(value))) => {
            let url = Cow::Borrowed(value);
            std::env::set_var("BIG_QUERY_AUTH_URL", url.as_ref());
            gcp_bigquery_client::Client::with_workload_identity(false)
                .await
                .unwrap()
        }
        Some(WorkloadIdentityAuth(Secret::FromEnvironment { variable })) => {
            let url: Cow<'_, String> = Cow::Owned(environment.read(variable)?);
            std::env::set_var("BIG_QUERY_AUTH_URL", url.as_ref());
            gcp_bigquery_client::Client::with_workload_identity(false)
                .await
                .unwrap()
        }
        None => match &args.connection_settings.service_key {
            Some(ServiceKey(Secret::Plain(value))) => {
                let service_key = Cow::Borrowed(value);
                let service_account_key =
                    yup_oauth2::parse_service_account_key(service_key.as_str()).unwrap();
                gcp_bigquery_client::Client::from_service_account_key(service_account_key, false)
                    .await
                    .unwrap()
            }
            Some(ServiceKey(Secret::FromEnvironment { variable })) => {
                let service_key: Cow<'_, String> = Cow::Owned(environment.read(variable)?);
                let service_account_key =
                    yup_oauth2::parse_service_account_key(service_key.as_str()).unwrap();
                gcp_bigquery_client::Client::from_service_account_key(service_account_key, false)
                    .await
                    .unwrap()
            }
            None => {
                return Err(anyhow::anyhow!(
                    "Neither Workload Identity Auth URL or Service key is provided"
                ));
            }
        },
    };
    // let service_key = match &args.connection_settings.service_key {
    //     Some(ServiceKey(Secret::Plain(value))) => Cow::Borrowed(value),
    //     Some(ServiceKey(Secret::FromEnvironment { variable })) => Cow::Owned(environment.read(variable)?),
    // };

    // let workload_identity_auth = match &args.connection_settings.workload_identity_auth {
    //     WorkloadIdentityAuth(Secret::Plain(value)) => Cow::Borrowed(value),
    //     WorkloadIdentityAuth(Secret::FromEnvironment { variable }) => Cow::Owned(environment.read(variable)?),
    // };

    let project_id_ = match &args.connection_settings.project_id {
        ProjectId(Secret::Plain(value)) => Cow::Borrowed(value),
        ProjectId(Secret::FromEnvironment { variable }) => Cow::Owned(environment.read(variable)?),
    };

    let dataset_id_ = match &args.connection_settings.dataset_id {
        DatasetId(Secret::Plain(value)) => Cow::Borrowed(value),
        DatasetId(Secret::FromEnvironment { variable }) => Cow::Owned(environment.read(variable)?),
    };

    // let service_account_key = yup_oauth2::parse_service_account_key(service_key.as_str()).unwrap();

    let project_id = project_id_.as_str();
    let dataset_id = dataset_id_.as_str();

    let schema_name = format!("{project_id}.{dataset_id}");
    let database_name = schema_name.clone();

    // let bigquery_client = match
    // BIG_QUERY_AUTH_URL

    // let big_query_auth_url = match std::env::var("HASURA_BIGQUERY_WORKLOAD_IDENTITY_AUTH") {
    //     Ok(val) => val,
    //     Err(_) => {
    //         return Err(anyhow::anyhow!(
    //             "Environment variable HASURA_BIGQUERY_WORKLOAD_IDENTITY_AUTH not set"
    //         ))
    //     }
    // };

    // std::env::set_var("BIG_QUERY_AUTH_URL", workload_identity_auth.as_ref());

    // if

    // let bigquery_client_auth = gcp_bigquery_client::Client::with_workload_identity(true).await.unwrap();

    // // Init BigQuery client
    // let bigquery_client =
    //     gcp_bigquery_client::Client::from_service_account_key(service_account_key, false)
    //         .await
    //         .unwrap();

    // get scalar_types

    let types_query = format!(
        "select coalesce(data_type, '') as data_type from {project_id}.{dataset_id}.INFORMATION_SCHEMA.COLUMN_FIELD_PATHS"
    );

    let types_row = bigquery_client
        .job()
        .query(project_id, QueryRequest::new(types_query))
        .await
        .unwrap();

    let types_query_response = types_row.query_response().clone();
    let empty_tablerow = vec![TableRow::default()];
    let empty_tablecell = &vec![TableCell::default()];
    let empty_string_value = &serde_json::Value::String(String::new());

    // let types_query = types_query_response.rows.unwrap_or_default();

    //TODO(PY): too many unwraps!
    let types = types_query_response
        .rows
        .as_ref()
        .unwrap_or_else(|| empty_tablerow.as_ref())
        .iter()
        .map(|row| TypeItem {
            name: serde_json::from_value(
                row.columns
                    .as_ref()
                    .unwrap_or(empty_tablecell)
                    .iter()
                    .next()
                    .unwrap()
                    .value
                    .as_ref()
                    .unwrap_or(empty_string_value)
                    .to_owned(),
            )
            .unwrap(),
        })
        .collect::<Vec<_>>();

    let scalar_types = get_scalar_types(&types, schema_name);

    let config_query_string = CONFIGURATION_QUERY.to_string();

    let config_query_string_with_database_name: String =
        config_query_string.replace("HASURA_DATABASE_NAME_PLACEHOLDER", database_name.as_str()); //TODO(PY): what is a safe name to provide as a variable name?

    let config_query_with_schema_name = config_query_string_with_database_name
        .replace("HASURA_DATABASE_SCHEMA_PLACEHOLDER", dataset_id);

    let tables_query_request = QueryRequest::new(config_query_with_schema_name);

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
                        serde_json::from_str::<TablesInfo>(&str).map_err(|err| {
                            format!("Failed to deserialize TablesInfo from JSON: {err}")
                        })
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
        connection_settings: connection_settings::DatabaseConnectionSettings {
            service_key: args.connection_settings.service_key.clone(),
            workload_identity_auth: args.connection_settings.workload_identity_auth.clone(),
            project_id: args.connection_settings.project_id.clone(),
            dataset_id: args.connection_settings.dataset_id.clone(),
        },
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
    configuration_dir: impl AsRef<Path> + Send,
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

    let parsed_config: ParsedConfiguration = serde_json::from_str(&configuration_file_contents)
        .map_err(|error| ParseConfigurationError::ParseError {
            file_path: configuration_file.clone(),
            line: error.line(),
            column: error.column(),
            message: error.to_string(),
        })?;

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

#[derive(Deserialize, Debug)]
struct TypeItem {
    name: ScalarTypeName,
}

fn get_aggregate_functions_for_type(
    type_representation: &Option<database::BigQueryType>,
    type_name: &ScalarTypeName,
) -> BTreeMap<AggregateFunctionName, database::AggregateFunction> {
    let mut aggregate_functions = BTreeMap::new();

    match type_representation {
        Some(type_rep) => {
            if matches!(
                type_rep,
                database::BigQueryType::Int64
                    | database::BigQueryType::Float64
                    | database::BigQueryType::Numeric
                    | database::BigQueryType::BigNumeric
                    | database::BigQueryType::String
                    | database::BigQueryType::Date
                    | database::BigQueryType::Datetime
                    | database::BigQueryType::Timestamp
                    | database::BigQueryType::Time
            ) {
                aggregate_functions.insert(
                    AggregateFunctionName::new("MIN".into()),
                    database::AggregateFunction {
                        return_type: TypeName::new(type_name.as_str().into()),
                    },
                );
                aggregate_functions.insert(
                    AggregateFunctionName::new("MAX".into()),
                    database::AggregateFunction {
                        return_type: TypeName::new(type_name.as_str().into()),
                    },
                );
            }

            if matches!(
                type_rep,
                database::BigQueryType::Int64
                    | database::BigQueryType::Float64
                    | database::BigQueryType::Numeric
                    | database::BigQueryType::BigNumeric
            ) {
                aggregate_functions.insert(
                    AggregateFunctionName::new("AVG".into()),
                    database::AggregateFunction {
                        return_type: TypeName::new(type_name.as_str().into()),
                    },
                );
                aggregate_functions.insert(
                    AggregateFunctionName::new("SUM".into()),
                    database::AggregateFunction {
                        return_type: TypeName::new(type_name.as_str().into()),
                    },
                );
            };
        }
        None => {}
    }

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
        let type_rep = get_type_representation(type_item);
        let type_name_str = match type_rep.clone() {
            Some(typerep) => ndc_models::TypeName::from(typerep),
            None => TypeName::new(SmolStr::new("any")),
        };
        let scalar_type_name = ScalarTypeName::new(type_name_str);

        scalar_types.insert(
            scalar_type_name.clone(),
            database::ScalarType {
                type_name: scalar_type_name.clone(),
                schema_name: schema.clone(),
                comparison_operators: get_comparison_operators_for_type(
                    &type_rep,
                    &scalar_type_name,
                ),
                aggregate_functions: get_aggregate_functions_for_type(&type_rep, &scalar_type_name),
                description: None,
                type_representation: type_rep.clone(),
            },
        );
    }

    database::ScalarTypes(scalar_types)
}

// we hard code these, essentially
// we look up available types in `sys.types` but hard code their behaviour by looking them up below
// categories taken from https://learn.microsoft.com/en-us/sql/t-sql/data-types/data-types-transact-sql
fn get_comparison_operators_for_type(
    type_representation: &Option<database::BigQueryType>,
    type_name: &ScalarTypeName,
) -> BTreeMap<ComparisonOperatorName, database::ComparisonOperator> {
    let mut comparison_operators = BTreeMap::new();

    match type_representation {
        Some(type_rep) => {
            if !matches!(
                type_rep,
                database::BigQueryType::Array(_)
                    | database::BigQueryType::Bytes
                    | database::BigQueryType::Json
                    | database::BigQueryType::Geography
                    | database::BigQueryType::Struct(_)
                    | database::BigQueryType::Range(_)
            ) {
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
            }

            if matches!(
                type_rep,
                database::BigQueryType::String | database::BigQueryType::Bytes
            ) {
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

            if !matches!(
                type_rep,
                database::BigQueryType::Array(_)
                    | database::BigQueryType::Json
                    | database::BigQueryType::Bytes
                    | database::BigQueryType::Geography
                    | database::BigQueryType::Struct(_)
                    | database::BigQueryType::Range(_)
            ) {
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
        }
        None => {}
    }

    comparison_operators
}

fn get_type_representation(type_item: &TypeItem) -> Option<database::BigQueryType> {
    match type_item.name.as_str().to_lowercase().as_str() {
        "string" => Some(database::BigQueryType::String),
        "bytes" => Some(database::BigQueryType::Bytes),
        "int64" => Some(database::BigQueryType::Int64),
        "float64" => Some(database::BigQueryType::Float64),
        "bool" => Some(database::BigQueryType::Boolean),
        "numeric" => Some(database::BigQueryType::Numeric),
        "bignumeric" => Some(database::BigQueryType::BigNumeric),
        "geography" => Some(database::BigQueryType::Geography),
        "date" => Some(database::BigQueryType::Date),
        "datetime" => Some(database::BigQueryType::Datetime),
        "time" => Some(database::BigQueryType::Time),
        "timestamp" => Some(database::BigQueryType::Timestamp),
        "json" => Some(database::BigQueryType::Json),
        t if t.starts_with("range<") => {
            let inner_type = t.trim_start_matches("range<").trim_end_matches('>');
            let range_type = match inner_type {
                "datetime" => database::TypeRange::Datetime,
                "timestamp" => database::TypeRange::Timestamp,
                _ => database::TypeRange::Date, // Default to Date for everything else
            };
            Some(database::BigQueryType::Range(Box::new(range_type)))
        }
        t if t.starts_with("array<") => {
            let inner_type = t.trim_start_matches("array<").trim_end_matches('>');
            let inner_repr = get_type_representation(&TypeItem {
                name: ScalarTypeName::new(inner_type.into()),
            })
            .unwrap_or(database::BigQueryType::String);
            Some(database::BigQueryType::Array(Box::new(inner_repr)))
        }
        t if t.starts_with("struct<") => {
            let fields = t.trim_start_matches("struct<").trim_end_matches('>');
            let mut struct_fields = BTreeMap::new();
            for field in fields.split(',') {
                let parts: Vec<&str> = field.split_whitespace().collect();
                if parts.len() == 2 {
                    let field_name = parts[0].to_string();
                    let field_type = get_type_representation(&TypeItem {
                        name: ScalarTypeName::new(parts[1].into()),
                    })
                    .unwrap_or(database::BigQueryType::String);
                    struct_fields.insert(field_name, Box::new(field_type));
                }
            }
            Some(database::BigQueryType::Struct(struct_fields))
        }
        _ => None,
    }
}
