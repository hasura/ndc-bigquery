//! Internal Configuration and state for our connector.

use thiserror::Error;

use query_engine_execution::metrics;
use query_engine_metadata::metadata;

mod version1;

use tracing::{info_span, Instrument};

pub use version1::{
    configure,
    single_connection_uri, // for tests only
    validate_raw_configuration,
    Configuration,
    ConfigurationError,
    PoolSettings,
    RawConfiguration,
};

pub const CURRENT_VERSION: u32 = 1;

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
    pub metadata: &'a metadata::Metadata,
    pub aggregate_functions: &'a metadata::AggregateFunctions,
}

impl<'a> version1::Configuration {
    /// Apply the common interpretations on the Configuration API type into an RuntimeConfiguration.
    /// This means things like specializing the configuration to the particular region the NDC runs in,
    pub fn as_runtime_configuration(
        self: &'a Configuration,
    ) -> Result<RuntimeConfiguration<'a>, ConfigurationError> {
        Ok(RuntimeConfiguration {
            aggregate_functions: &self.config.aggregate_functions,
            metadata: &self.config.metadata,
        })
    }
}

/// State for our connector.
#[derive(Clone)]
pub struct State {
    pub metrics: metrics::Metrics,
    pub bigquery_client: gcp_bigquery_client::Client,
}

/// Create a connection pool and wrap it inside a connector State.
pub async fn create_state(
    _configuration: &Configuration,
    metrics_registry: &mut prometheus::Registry,
) -> Result<State, InitializationError> {
    let metrics = async {
        let metrics_inner = metrics::Metrics::initialize(metrics_registry)
            .map_err(InitializationError::MetricsError)?;
        Ok(metrics_inner)
    }
    .instrument(info_span!("Setup metrics"))
    .await?;

    let service_account_key_json = std::env::var("HASURA_BIGQUERY_SERVICE_KEY").unwrap();

    let service_account_key =
        yup_oauth2::parse_service_account_key(service_account_key_json).unwrap();

    // Init BigQuery client
    let bigquery_client =
        gcp_bigquery_client::Client::from_service_account_key(service_account_key, false)
            .await
            .unwrap();

    Ok(State {
        metrics,
        bigquery_client,
    })
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
