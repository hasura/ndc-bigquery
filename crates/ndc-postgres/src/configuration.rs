/// Configuration and state for our connector.
use ndc_hub::connector;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use thiserror::Error;

/// User configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeploymentConfiguration {
    pub version: u32,
    pub tables: query_engine::metadata::TablesInfo,
    pub postgres_database_url: String,
}

/// State for our connector.
#[derive(Debug, Clone)]
pub struct State {
    pub pool: PgPool,
}

/// Validate the user configuration.
pub async fn validate_raw_configuration(
    configuration: &DeploymentConfiguration,
) -> Result<DeploymentConfiguration, connector::ConfigurationError> {
    if configuration.version != 1 {
        return Err(connector::ConfigurationError::Other(
            ConfigurationError::InvalidConfigVersion(configuration.version).into(),
        ));
    }
    Ok(configuration.clone())
}

/// Create a connection pool and wrap it inside a connector State.
pub async fn create_state(
    configuration: &DeploymentConfiguration,
) -> Result<State, connector::InitializationError> {
    let pool = create_pool(configuration).await.map_err(|e| {
        connector::InitializationError::Other(InitializationError::UnableToCreatePool(e).into())
    })?;
    Ok(State { pool })
}

/// Create a connection pool with default settings.
async fn create_pool(configuration: &DeploymentConfiguration) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(50)
        .connect(&configuration.postgres_database_url)
        .await
}

/// User configuration error.
#[derive(Debug, Error)]
enum ConfigurationError {
    #[error("invalid configuration version, expected 1, got {0}")]
    InvalidConfigVersion(u32),
}

/// State initialization error.
#[derive(Debug, Error)]
enum InitializationError {
    #[error("unable to initialize connection pool: {0}")]
    UnableToCreatePool(sqlx::Error),
}
