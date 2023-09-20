//! Internal Configuration and state for our connector.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnection, PgPool, PgPoolOptions};
use sqlx::{Connection, Executor, Row};
use std::collections::BTreeMap;
use thiserror::Error;

use ndc_sdk::connector;

use query_engine_execution::metrics;
use query_engine_metadata::metadata;

const CURRENT_VERSION: u32 = 1;

/// Initial configuration, just enough to connect to a database and elaborate a full
/// 'Configuration'.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct RawConfiguration {
    // Which version of the configuration format are we using
    pub version: u32,
    // Connection string for a Postgres-compatible database
    pub connection_uris: ConnectionUris,
    #[serde(skip_serializing_if = "PoolSettings::is_default")]
    #[serde(default)]
    pub pool_settings: PoolSettings,
    #[serde(default)]
    pub metadata: metadata::Metadata,
    #[serde(default)]
    pub aggregate_functions: metadata::AggregateFunctions,
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
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    /// Routing table which relates the regions the NDC may be deployed in with the regions that
    /// the database is deployed, in order of preference.
    pub region_routing: BTreeMap<HasuraRegionName, Vec<RegionName>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ConnectionUris {
    SingleRegion(Vec<String>),
    MultiRegion(MultipleRegionsConnectionUrls),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct MultipleRegionsConnectionUrls {
    pub writes: BTreeMap<RegionName, Vec<String>>,
    pub reads: BTreeMap<RegionName, Vec<String>>,
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

/// Select a single connection uri to use, given an application region.
///
/// Currently we simply select the first specified connection uri, and in the case of multi-region,
/// only the first from the list of read-only servers.
///
/// Eventually we want to support load-balancing between multiple read-replicas within a region,
/// and then we'll be passing the full list of connection uris to the connection pool.
pub fn select_connection_url<'a>(
    urls: &'a ConnectionUris,
    region_routing: &BTreeMap<HasuraRegionName, Vec<RegionName>>,
) -> Result<&'a str, ConfigurationError> {
    match &urls {
        ConnectionUris::SingleRegion(urls) => Ok(&urls[0]),
        ConnectionUris::MultiRegion(MultipleRegionsConnectionUrls { reads, .. }) => {
            let region = route_region(region_routing)?;
            let urls = reads
                .get(region)
                .ok_or_else(|| ConfigurationError::UnableToMapApplicationRegion(region.clone()))?;
            Ok(&urls[0])
        }
    }
}

/// Select the database region to use, observing the DDN_REGION environment variable.
pub fn route_region(
    region_routing: &BTreeMap<HasuraRegionName, Vec<RegionName>>,
) -> Result<&RegionName, ConfigurationError> {
    let ddn_region = HasuraRegionName(
        std::env::var("DDN_REGION").or(Err(ConfigurationError::DdnRegionIsNotSet))?,
    );
    let connection_uris = &region_routing
        .get(&ddn_region)
        .ok_or_else(|| ConfigurationError::UnableToMapHasuraRegion(ddn_region.clone()))?;

    Ok(&connection_uris[0])
}

/// Select the first available connection uri. Suitable for when hasura regions are not yet mapped
/// to application regions.
pub fn select_first_connection_url(urls: &ConnectionUris) -> &str {
    match &urls {
        ConnectionUris::SingleRegion(urls) => &urls[0],
        ConnectionUris::MultiRegion(MultipleRegionsConnectionUrls { reads, .. }) => reads
            .first_key_value()
            .expect("No regions are defined (Guarded by validate_raw_configuration)")
            .1[0]
            .as_str(),
    }
}

/// Settings for the PostgreSQL connection pool
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PoolSettings {
    /// maximum number of pool connections
    #[serde(default = "max_connection_default")]
    max_connections: u32,
    /// timeout for acquiring a connection from the pool (seconds)
    #[serde(default = "pool_timeout_default")]
    pool_timeout: u64,
    /// idle timeout for releasing a connection from the pool (seconds)
    #[serde(default = "idle_timeout_default")]
    idle_timeout: Option<u64>,
    /// maximum lifetime for an individual connection (seconds)
    #[serde(default = "connection_lifetime_default")]
    connection_lifetime: Option<u64>,
}

impl PoolSettings {
    fn is_default(&self) -> bool {
        *self == PoolSettings::default()
    }
}

/// <https://hasura.io/docs/latest/api-reference/syntax-defs/#pgpoolsettings>
impl Default for PoolSettings {
    fn default() -> PoolSettings {
        PoolSettings {
            max_connections: 50,
            pool_timeout: 600,
            idle_timeout: Some(180),
            connection_lifetime: Some(600),
        }
    }
}

// for serde default //
fn max_connection_default() -> u32 {
    PoolSettings::default().max_connections
}
fn pool_timeout_default() -> u64 {
    PoolSettings::default().pool_timeout
}
fn idle_timeout_default() -> Option<u64> {
    PoolSettings::default().idle_timeout
}
fn connection_lifetime_default() -> Option<u64> {
    PoolSettings::default().connection_lifetime
}
///////////////////////

impl RawConfiguration {
    pub fn empty() -> Self {
        Self {
            version: CURRENT_VERSION,
            connection_uris: ConnectionUris::SingleRegion(vec![]),
            pool_settings: PoolSettings::default(),
            metadata: metadata::Metadata::default(),
            aggregate_functions: metadata::AggregateFunctions::default(),
        }
    }
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

    match &rawconfiguration.connection_uris {
        ConnectionUris::SingleRegion(urls) if urls.is_empty() => {
            Err(connector::ValidateError::ValidateError(vec![
                connector::InvalidRange {
                    path: vec![connector::KeyOrIndex::Key("connection_uris".into())],
                    message: "At least one database url must be specified".to_string(),
                },
            ]))
        }
        ConnectionUris::MultiRegion(MultipleRegionsConnectionUrls { reads, writes }) => {
            let reads_empty_err = if reads.is_empty() {
                vec![connector::InvalidRange {
                    path: vec![
                        connector::KeyOrIndex::Key("connection_uris".into()),
                        connector::KeyOrIndex::Key("reads".into()),
                    ],
                    message: "At least one 'reads' region must be specified".to_string(),
                }]
            } else {
                vec![]
            };
            let reads_errs = reads
                .iter()
                .flat_map(|(RegionName(region), urls)| {
                    if urls.is_empty() {
                        vec![connector::InvalidRange {
                            path: vec![
                                connector::KeyOrIndex::Key("connection_uris".into()),
                                connector::KeyOrIndex::Key("reads".into()),
                                connector::KeyOrIndex::Key(region.into()),
                            ],
                            message: "At least one database url must be specified".to_string(),
                        }]
                    } else {
                        vec![]
                    }
                })
                .collect::<Vec<connector::InvalidRange>>();
            let writes_errs = writes
                .iter()
                .flat_map(|(RegionName(region), urls)| {
                    if urls.is_empty() {
                        vec![connector::InvalidRange {
                            path: vec![
                                connector::KeyOrIndex::Key("connection_uris".into()),
                                connector::KeyOrIndex::Key("writes".into()),
                                connector::KeyOrIndex::Key(region.into()),
                            ],
                            message: "At least one database url must be specified".to_string(),
                        }]
                    } else {
                        vec![]
                    }
                })
                .collect::<Vec<connector::InvalidRange>>();

            let mut errs = vec![];

            errs.extend(reads_empty_err);
            errs.extend(reads_errs);
            errs.extend(writes_errs);

            if !errs.is_empty() {
                Err(connector::ValidateError::ValidateError(errs))
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }?;

    // Collect the regions that have been specified, to enable geo-localised deployments.
    let (write_regions, read_regions) = match &rawconfiguration.connection_uris {
        ConnectionUris::MultiRegion(MultipleRegionsConnectionUrls { writes, reads }) => (
            writes.keys().cloned().collect::<Vec<_>>(),
            reads.keys().cloned().collect::<Vec<_>>(),
        ),
        ConnectionUris::SingleRegion(_) => (vec![], vec![]),
    };

    // region routing is provided by the metadata build service before the
    // agent is deployed, so we don't need to try and calculate it here.
    let region_routing = BTreeMap::new();

    Ok(Configuration {
        config: rawconfiguration.clone(),
        write_regions,
        read_regions,
        region_routing,
    })
}

/// Construct the deployment configuration by introspecting the database.
pub async fn configure(
    args: &RawConfiguration,
    configuration_query: &str,
) -> Result<RawConfiguration, connector::UpdateConfigurationError> {
    let url = select_first_connection_url(&args.connection_uris);

    let mut connection = PgConnection::connect(url)
        .await
        .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    let row = connection
        .fetch_one(configuration_query)
        .await
        .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    let tables: metadata::TablesInfo = serde_json::from_value(row.get(0))
        .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    let aggregate_functions: metadata::AggregateFunctions = serde_json::from_value(row.get(1))
        .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    Ok(RawConfiguration {
        version: 1,
        connection_uris: args.connection_uris.clone(),
        pool_settings: args.pool_settings.clone(),
        metadata: metadata::Metadata {
            tables,
            native_queries: args.metadata.native_queries.clone(),
        },
        aggregate_functions,
    })
}

/// A configuration type, tailored to the needs of the query/mutation/explain methods (i.e., those
/// not to do with configuration management).
///
/// This separation also decouples the implementation from things like API versioning concerns
/// somewhat.
///
/// Since the RuntimeConfiguration is reconstructed from a Configuration at every method call, and
/// since it consists of a sub-selection of components from the full Configuration, the fields are
/// borrowed rather than owned.
#[derive(Debug, PartialEq)]
pub struct RuntimeConfiguration<'a> {
    pub connection_uris: &'a str,
    pub metadata: &'a metadata::Metadata,
    pub aggregate_functions: &'a metadata::AggregateFunctions,
}

impl Configuration {
    /// Apply the common interpretations on the Configuration API type into an RuntimeConfiguration.
    /// This means things like specializing the configuration to the particular region the NDC runs in,
    pub fn as_runtime_configuration(
        self: &Configuration,
    ) -> Result<RuntimeConfiguration, ConfigurationError> {
        // Look-up region-specific connection strings.
        let connection_uris =
            select_connection_url(&self.config.connection_uris, &self.region_routing)?;

        Ok(RuntimeConfiguration {
            connection_uris,
            aggregate_functions: &self.config.aggregate_functions,
            metadata: &self.config.metadata,
        })
    }
}

/// State for our connector.
#[derive(Debug, Clone)]
pub struct State {
    pub pool: PgPool,
    pub metrics: metrics::Metrics,
}

/// Create a connection pool and wrap it inside a connector State.
pub async fn create_state(
    configuration: &Configuration,
    metrics_registry: &mut prometheus::Registry,
) -> Result<State, InitializationError> {
    let pool = create_pool(configuration).await?;

    let metrics = metrics::Metrics::initialize(metrics_registry)
        .map_err(InitializationError::MetricsError)?;
    metrics.set_pool_options_metrics(pool.options());

    Ok(State { pool, metrics })
}

/// Create a connection pool with default settings.
/// - <https://docs.rs/sqlx/latest/sqlx/pool/struct.PoolOptions.html>
async fn create_pool(configuration: &Configuration) -> Result<PgPool, InitializationError> {
    let url = select_connection_url(
        &configuration.config.connection_uris,
        &configuration.region_routing,
    )
    .map_err(InitializationError::ConfigurationError)?;

    let pool_settings = &configuration.config.pool_settings;

    PgPoolOptions::new()
        .max_connections(pool_settings.max_connections)
        .acquire_timeout(std::time::Duration::from_secs(pool_settings.pool_timeout))
        .idle_timeout(
            pool_settings
                .idle_timeout
                .map(std::time::Duration::from_secs),
        )
        .max_lifetime(
            pool_settings
                .connection_lifetime
                .map(std::time::Duration::from_secs),
        )
        .connect(url)
        .await
        .map_err(InitializationError::UnableToCreatePool)
}

/// State initialization error.
#[derive(Debug, Error)]
pub enum InitializationError {
    #[error("unable to initialize connection pool: {0}")]
    UnableToCreatePool(sqlx::Error),
    #[error("error initializing metrics: {0}")]
    MetricsError(metrics::Error),
    #[error("{0}")]
    ConfigurationError(ConfigurationError),
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
