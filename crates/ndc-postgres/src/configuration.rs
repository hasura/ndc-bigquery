//! Configuration and state for our connector.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnection, PgPool, PgPoolOptions};
use sqlx::{Connection, Executor, Row};
use thiserror::Error;

use ndc_sdk::connector;

use super::metrics;

const CURRENT_VERSION: u32 = 1;
const CONFIGURATION_QUERY: &str = include_str!("configuration.sql");

/// User configuration.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct DeploymentConfiguration {
    // Which version of the configuration format are we using
    pub version: u32,
    // Connection string for a Postgres-compatible database
    pub postgres_database_url: String,
    #[serde(default)]
    pub metadata: query_engine::metadata::Metadata,
    #[serde(default)]
    pub aggregate_functions: query_engine::metadata::AggregateFunctions,
}

impl DeploymentConfiguration {
    pub fn empty() -> Self {
        Self {
            version: CURRENT_VERSION,
            postgres_database_url: "".into(),
            metadata: query_engine::metadata::Metadata::default(),
            aggregate_functions: query_engine::metadata::AggregateFunctions::default(),
        }
    }
}

/// State for our connector.
#[derive(Debug, Clone)]
pub struct State {
    pub pool: PgPool,
    pub metrics: metrics::Metrics,
}

/// Validate the user configuration.
pub async fn validate_raw_configuration(
    configuration: &DeploymentConfiguration,
) -> Result<DeploymentConfiguration, connector::ValidateError> {
    if configuration.version != 1 {
        return Err(connector::ValidateError::ValidateError(vec![
            connector::InvalidRange {
                path: vec![connector::KeyOrIndex::Key("version".into())],
                message: format!(
                    "invalid configuration version, expected 1, got {0}",
                    configuration.version
                ),
            },
        ]));
    }
    Ok(configuration.clone())
}

/// Create a connection pool and wrap it inside a connector State.
pub async fn create_state(
    configuration: &DeploymentConfiguration,
    metrics_registry: &mut prometheus::Registry,
) -> Result<State, connector::InitializationError> {
    let pool = create_pool(configuration).await.map_err(|e| {
        connector::InitializationError::Other(InitializationError::UnableToCreatePool(e).into())
    })?;

    let metrics = metrics::initialise_metrics(metrics_registry).await?;
    Ok(State { pool, metrics })
}

/// Create a connection pool with default settings.
async fn create_pool(configuration: &DeploymentConfiguration) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(50)
        .connect(&configuration.postgres_database_url)
        .await
}

/// Construct the deployment configuration by introspecting the database.
pub async fn configure(
    args: &DeploymentConfiguration,
) -> Result<DeploymentConfiguration, connector::UpdateConfigurationError> {
    let mut connection = PgConnection::connect(&args.postgres_database_url)
        .await
        .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    let row = connection
        .fetch_one(CONFIGURATION_QUERY)
        .await
        .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    let tables: query_engine::metadata::TablesInfo = serde_json::from_value(row.get(0))
        .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    let aggregate_functions: query_engine::metadata::AggregateFunctions =
        serde_json::from_value(row.get(1))
            .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    Ok(DeploymentConfiguration {
        version: 1,
        postgres_database_url: args.postgres_database_url.clone(),
        metadata: query_engine::metadata::Metadata {
            tables,
            native_queries: query_engine::metadata::NativeQueries::default(),
        },
        aggregate_functions,
    })
}

/// State initialization error.
#[derive(Debug, Error)]
pub enum InitializationError {
    #[error("unable to initialize connection pool: {0}")]
    UnableToCreatePool(sqlx::Error),
    #[error("error initializing Prometheus metrics: {0}")]
    PrometheusError(prometheus::Error),
}
