//! Transient state used by the connector.
//!
//! This is initialized on startup.

use thiserror::Error;
use tracing::{info_span, Instrument};

// use ndc_bigquery_configuration::PoolSettings;
use ndc_bigquery_configuration::{version1, ConfigurationError};
// use super::configuration::{Configuration, ConfigurationError};
// use query_engine_execution::database_info::{self, DatabaseInfo, DatabaseVersion};
use query_engine_execution::metrics;

/// State for our connector.
#[derive(Clone)]
pub struct State {
    pub metrics: metrics::Metrics,
    pub bigquery_client: gcp_bigquery_client::Client,
}

/// Create a connection pool and wrap it inside a connector State.
pub async fn create_state(
    // _configuration: &Configuration,
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
    MetricsError(prometheus::Error),
    #[error("{0}")]
    ConfigurationError(ConfigurationError),
}
