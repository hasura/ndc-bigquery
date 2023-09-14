//! Configuration and state for our connector.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnection, PgPool, PgPoolOptions};
use sqlx::{Connection, Executor, Row};
use std::collections::BTreeMap;
use thiserror::Error;

use ndc_sdk::connector;

use super::metrics;

const CURRENT_VERSION: u32 = 1;

/// Initial configuration, just enough to connect to a database and elaborate a full
/// 'Configuration'.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct RawConfiguration {
    // Which version of the configuration format are we using
    pub version: u32,
    // Connection string for a Postgres-compatible database
    pub postgres_database_url: PostgresDatabaseUrls,
    #[serde(default)]
    pub metadata: query_engine::metadata::Metadata,
    #[serde(default)]
    pub aggregate_functions: query_engine::metadata::AggregateFunctions,
}

/// User configuration, elaborated from a 'RawConfiguration'.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct Configuration {
    pub config: RawConfiguration,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub write_regions: Vec<RegionName>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub read_regions: Vec<RegionName>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum PostgresDatabaseUrls {
    SingleRegion(String),
    MultiRegion(MultipleRegionsConnectionUrls),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct MultipleRegionsConnectionUrls {
    pub writes: BTreeMap<RegionName, Vec<String>>,
    pub reads: BTreeMap<RegionName, Vec<String>>,
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Ord, Deserialize, Serialize, JsonSchema)]
pub struct RegionName(String);

impl RawConfiguration {
    pub fn empty() -> Self {
        Self {
            version: CURRENT_VERSION,
            postgres_database_url: PostgresDatabaseUrls::SingleRegion("".into()),
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
    rawconfiguration: &RawConfiguration,
) -> Result<Configuration, connector::ValidateError> {
    if rawconfiguration.version != 1 {
        return Err(connector::ValidateError::ValidateError(vec![
            connector::InvalidRange {
                path: vec![connector::KeyOrIndex::Key("version".into())],
                message: format!(
                    "invalid configuration version, expected 1, got {0}",
                    rawconfiguration.version
                ),
            },
        ]));
    }

    // Collect the regions that have been specified, to enable geo-localised deployments.
    let (write_regions, read_regions) = match &rawconfiguration.postgres_database_url {
        PostgresDatabaseUrls::MultiRegion(MultipleRegionsConnectionUrls { writes, reads }) => (
            writes.keys().cloned().collect::<Vec<_>>(),
            reads.keys().cloned().collect::<Vec<_>>(),
        ),
        PostgresDatabaseUrls::SingleRegion(_) => (vec![], vec![]),
    };
    Ok(Configuration {
        config: rawconfiguration.clone(),
        write_regions,
        read_regions,
    })
}

/// Create a connection pool and wrap it inside a connector State.
pub async fn create_state(
    configuration: &Configuration,
    metrics_registry: &mut prometheus::Registry,
) -> Result<State, connector::InitializationError> {
    let pool = create_pool(configuration).await.map_err(|e| {
        connector::InitializationError::Other(InitializationError::UnableToCreatePool(e).into())
    })?;

    let metrics = metrics::Metrics::initialize(metrics_registry)?;
    metrics.set_pool_options_metrics(pool.options());

    Ok(State { pool, metrics })
}

/// Create a connection pool with default settings.
async fn create_pool(configuration: &Configuration) -> Result<PgPool, sqlx::Error> {
    let url = match &configuration.config.postgres_database_url {
        PostgresDatabaseUrls::SingleRegion(url) => url,
        PostgresDatabaseUrls::MultiRegion(_) => todo!(),
    };

    PgPoolOptions::new().max_connections(50).connect(url).await
}

/// Construct the deployment configuration by introspecting the database.
pub async fn configure(
    args: &RawConfiguration,
    configuration_query: &str,
) -> Result<RawConfiguration, connector::UpdateConfigurationError> {
    let url = match &args.postgres_database_url {
        PostgresDatabaseUrls::SingleRegion(url) => url,
        PostgresDatabaseUrls::MultiRegion(_) => todo!(),
    };

    let mut connection = PgConnection::connect(url)
        .await
        .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    let row = connection
        .fetch_one(configuration_query)
        .await
        .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    let tables: query_engine::metadata::TablesInfo = serde_json::from_value(row.get(0))
        .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    let aggregate_functions: query_engine::metadata::AggregateFunctions =
        serde_json::from_value(row.get(1))
            .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    Ok(RawConfiguration {
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
