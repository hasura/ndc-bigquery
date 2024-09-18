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

    if let Some(precise_return_type) = match type_name.as_str() {
        "tinyint" | "smallint" | "int16" => Some("smallint"),
        "int" | "int32" => Some("integer"),
        "bigint" | "int64" => Some("bigint"),
        "float" | "real" => Some("float"),
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
            "int16" | "smallint" => "smallint",
            "int" | "int32" | "integer" => "integer",
            "int64" | "bigint" => "bigint",
            "numeric" => "numeric",
            "float64" | "float" => "float",
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
