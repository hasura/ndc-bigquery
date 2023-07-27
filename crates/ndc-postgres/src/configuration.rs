use clap::Args;
/// Configuration and state for our connector.
use ndc_hub::connector;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::collections::HashMap;
use thiserror::Error;

/// User configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeploymentConfiguration {
    pub version: u32,
    pub tables: query_engine::metadata::TablesInfo,
    pub postgres_database_url: String,
}

/// Arguments for configuration?
#[derive(Clone, Args)]
pub struct ConfigureArgs {
    #[arg()]
    pub postgres_connection_string: String,
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

/// Connect to the db and fetch the tables?
/// Copied from the ndc-postgres repo.
pub async fn configure(
    args: &ConfigureArgs,
) -> Result<DeploymentConfiguration, connector::ConfigurationError> {
    let statement_string = "
        select
            json_object_agg(
                t.table_name,
                json_build_object(
                    'schema_name',
                    t.table_schema,
                    'table_name',
                    t.table_name,
                    'columns',
                    (select
                        json_agg(
                            json_build_object(
                                'name',
                                c.column_name,
                                'type',
                                ''
                            )
                        )
                    from information_schema.columns c
                    where
                      c.table_catalog = t.table_catalog and
                      c.table_name = t.table_name and
                      c.table_schema = t.table_schema
                    )
                )
            )
        from information_schema.tables t
        where t.table_schema = 'public'
        ";

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&args.postgres_connection_string)
        .await
        .map_err(|e| connector::ConfigurationError::Other(e.into()))?;

    let query = sqlx::query(statement_string);

    let value: sqlx::types::JsonValue = query
        .map(|row: sqlx::postgres::PgRow| row.get(0))
        .fetch_one(&pool)
        .await
        .map_err(|e| connector::ConfigurationError::Other(e.into()))?;

    let tables: HashMap<String, query_engine::metadata::TableInfo> = serde_json::from_value(value)
        .map_err(|e| connector::ConfigurationError::Other(e.into()))?;

    Ok(DeploymentConfiguration {
        version: 1,
        postgres_database_url: args.postgres_connection_string.clone(),
        tables: query_engine::metadata::TablesInfo(tables),
    })
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
