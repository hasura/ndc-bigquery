//! Transient state used by the connector.
//!
//! This is initialized on startup.

use thiserror::Error;
use tracing::{info_span, Instrument};

use query_engine_execution::metrics;

/// State for our connector.
#[derive(Clone)]
pub struct State {
    pub metrics: metrics::Metrics,
    pub bigquery_client: gcp_bigquery_client::Client,
    pub project_id: String,
    pub dataset_id: String,
}

/// Create a connection pool and wrap it inside a connector State.
pub async fn create_state(
    configuration: &ndc_bigquery_configuration::Configuration,
    metrics_registry: &mut prometheus::Registry,
) -> Result<State, InitializationError> {
    let metrics = async {
        let metrics_inner = metrics::Metrics::initialize(metrics_registry)
            .map_err(InitializationError::MetricsError)?;
        Ok(metrics_inner)
    }
    .instrument(info_span!("Setup metrics"))
    .await?;

    let bigquery_client = match &configuration.auth {
        (true, workload_identity_auth) => {
            std::env::set_var("BIG_QUERY_AUTH_URL", workload_identity_auth);
            gcp_bigquery_client::Client::with_workload_identity(false).await.unwrap()
        }
        (false, service_key) => {
            let service_account_key =
                yup_oauth2::parse_service_account_key(service_key.clone()).unwrap();
            gcp_bigquery_client::Client::from_service_account_key(service_account_key, false)
                .await
                .unwrap()
        }
    };

    Ok(State {
        metrics,
        bigquery_client,
        project_id: configuration.project_id.clone(),
        dataset_id: configuration.dataset_id.clone(),
    })
}

/// State initialization error.
#[derive(Debug, Error)]
pub enum InitializationError {
    #[error("unable to initialize connection pool: {0}")]
    UnableToCreatePool(sqlx::Error),
    #[error("error initializing metrics: {0}")]
    MetricsError(prometheus::Error),
}
