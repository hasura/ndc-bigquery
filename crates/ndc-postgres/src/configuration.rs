//! Internal Configuration and state for our connector.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnection, PgPool, PgPoolOptions};
use sqlx::{Connection, Executor, Row};
use std::collections::BTreeMap;
use thiserror::Error;

use ndc_sdk::connector;

use query_engine::metrics;

const CURRENT_VERSION: u32 = 1;

/// Initial configuration, just enough to connect to a database and elaborate a full
/// 'Configuration'.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct RawConfiguration {
    // Which version of the configuration format are we using
    pub version: u32,
    // Connection string for a Postgres-compatible database
    pub connection_uris: ConnectionUris,
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
pub struct HasuraRegionName(String);

impl std::fmt::Display for HasuraRegionName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let HasuraRegionName(region) = self;
        write!(f, "{}", region)
    }
}

/// Name of a region that database servers may live in. These regions are distinct from the regions
/// the ndc can live in, and they need not be related a priori.
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Ord, Deserialize, Serialize, JsonSchema)]
pub struct RegionName(String);

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

impl RawConfiguration {
    pub fn empty() -> Self {
        Self {
            version: CURRENT_VERSION,
            connection_uris: ConnectionUris::SingleRegion(vec![]),
            metadata: query_engine::metadata::Metadata::default(),
            aggregate_functions: query_engine::metadata::AggregateFunctions::default(),
        }
    }
}

/// Validate the user configuration.
pub async fn validate_raw_configuration(
    rawconfiguration: &RawConfiguration,
    raw_region_routing: &BTreeMap<String, Vec<String>>,
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
    // Region-routing table
    let region_routing = mk_region_routing_table(raw_region_routing);

    Ok(Configuration {
        config: rawconfiguration.clone(),
        write_regions,
        read_regions,
        region_routing,
    })
}

fn mk_region_routing_table(
    raw_region_routing: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<HasuraRegionName, Vec<RegionName>> {
    raw_region_routing
        .iter()
        .map(|(hasura_region, application_regions)| {
            (
                HasuraRegionName(hasura_region.to_string()),
                application_regions
                    .iter()
                    .map(|region| RegionName(region.to_string()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<HasuraRegionName, Vec<RegionName>>>()
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

    let tables: query_engine::metadata::TablesInfo = serde_json::from_value(row.get(0))
        .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    let aggregate_functions: query_engine::metadata::AggregateFunctions =
        serde_json::from_value(row.get(1))
            .map_err(|e| connector::UpdateConfigurationError::Other(e.into()))?;

    Ok(RawConfiguration {
        version: 1,
        connection_uris: args.connection_uris.clone(),
        metadata: query_engine::metadata::Metadata {
            tables,
            native_queries: query_engine::metadata::NativeQueries::default(),
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
/// Since the InternalConfiguration is reconstructed from a Configuration at every method call, and
/// since it consists of a sub-selection of components from the full Configuration, the fields are
/// borrowed rather than owned.
#[derive(Debug, PartialEq)]
pub struct InternalConfiguration<'a> {
    pub connection_uris: &'a str,
    pub metadata: &'a query_engine::metadata::Metadata,
    pub aggregate_functions: &'a query_engine::metadata::AggregateFunctions,
}

impl Configuration {
    /// Apply the common interpretations on the Configuration API type into an InternalConfiguration.
    /// This means things like specializing the configuration to the particular region the NDC runs in,
    pub fn as_internal(self: &Configuration) -> Result<InternalConfiguration, ConfigurationError> {
        // Look-up region-specific connection strings.
        let connection_uris =
            select_connection_url(&self.config.connection_uris, &self.region_routing)?;

        Ok(InternalConfiguration {
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
async fn create_pool(configuration: &Configuration) -> Result<PgPool, InitializationError> {
    let url = select_connection_url(
        &configuration.config.connection_uris,
        &configuration.region_routing,
    )
    .map_err(InitializationError::ConfigurationError)?;

    PgPoolOptions::new()
        .max_connections(50)
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
