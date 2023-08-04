//! Configuration and state for our connector.

use clap::Args;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use thiserror::Error;

use ndc_hub::connector;

/// User configuration.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct DeploymentConfiguration {
    pub version: u32,
    pub postgres_database_url: String,
    pub tables: query_engine::metadata::TablesInfo,
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
    // This requests the table configuration from the database.
    // The structure maps directly to `TableInfo`.
    //
    // It is very large. There are inline comments in the SQL to help understand
    // what's going on.
    //
    // TODO: This uses unqualified table (and view) and constraint names.
    // We will need to qualify them at some point. This makes the aliases seem
    // redundant, but they will change in the future.
    let statement_string = "
        select
            -- here, we construct the TableInfo directly in the query
            json_object_agg(
                -- the table alias, used for looking up the table (or view, or other relation)
                t.table_name,
                json_build_object(
                    -- the schema name
                    'schema_name',
                    t.table_schema,
                    -- the table name
                    'table_name',
                    t.table_name,
                    -- a mapping from column aliases to the column information
                    'columns',
                    -- this may be empty, in which case we coalesce with an empty object
                    coalesce(
                        (
                            select
                                json_object_agg(
                                    -- the column alias, used for looking up the column
                                    c.column_name,
                                    json_build_object(
                                        -- the column name
                                        'name',
                                        c.column_name
                                    )
                                )
                            from information_schema.columns c
                            where
                                c.table_catalog = t.table_catalog
                                and c.table_schema = t.table_schema
                                and c.table_name = t.table_name
                        ),
                        json_build_object()
                    ),
                    -- a mapping from the uniqueness constraint aliases to their details
                    'uniqueness_constraints',
                    -- this may be empty, in which case we coalesce with an empty object
                    coalesce(
                        (
                            select
                                json_object_agg(
                                    -- the name of the uniqueness constraint
                                    c.constraint_name,
                                    -- an array (parsed as a set) of the columns present in the constraint
                                    (
                                        select json_agg(cc.column_name)
                                        from information_schema.constraint_column_usage cc
                                        where
                                            cc.constraint_catalog = c.constraint_catalog
                                            and cc.constraint_schema = c.constraint_schema
                                            and cc.constraint_name = c.constraint_name
                                    )
                                )
                            from information_schema.table_constraints c
                            where
                                c.table_catalog = t.table_catalog
                                and c.table_schema = t.table_schema
                                and c.table_name = t.table_name
                                and c.constraint_type in ('PRIMARY KEY', 'UNIQUE')
                        ),
                        json_build_object()
                    ),
                    -- a mapping from the foreign relation aliases to their details
                    'foreign_relations',
                    -- this may be empty, in which case we coalesce with an empty object
                    coalesce(
                        (
                            select 
                                json_object_agg(
                                    -- the name of the foreign key constraint
                                    c.constraint_name,
                                    json_build_object(
                                        -- the name of the foreign relation
                                        'foreign_table',
                                        (
                                            select ft.table_name
                                            from information_schema.table_constraints ft
                                            where
                                                ft.constraint_catalog = rc.constraint_catalog
                                                and ft.constraint_schema = rc.constraint_schema
                                                and ft.constraint_name = rc.constraint_name
                                        ),
                                        -- a mapping from the local columns to the foreign columns
                                        'column_mapping',
                                        (
                                            select
                                                json_object_agg(fc.column_name, uc.column_name)
                                            from information_schema.key_column_usage fc
                                            join information_schema.key_column_usage uc
                                                on fc.position_in_unique_constraint = uc.ordinal_position
                                            where
                                                fc.constraint_catalog = rc.constraint_catalog
                                                and fc.constraint_schema = rc.constraint_schema
                                                and fc.constraint_name = rc.constraint_name
                                                and uc.constraint_catalog = rc.unique_constraint_catalog
                                                and uc.constraint_schema = rc.unique_constraint_schema
                                                and uc.constraint_name = rc.unique_constraint_name
                                        )
                                    )
                                )
                            from information_schema.table_constraints c
                            join information_schema.referential_constraints rc on 
                                c.constraint_catalog = rc.constraint_catalog
                                and c.constraint_schema = rc.constraint_schema
                                and c.constraint_name = rc.constraint_name
                            where
                                c.table_catalog = t.table_catalog
                                and c.table_schema = t.table_schema
                                and c.table_name = t.table_name
                                and c.constraint_type = 'FOREIGN KEY'
                        ),
                        json_build_object()
                    )
                )
            ) tables
        from information_schema.tables t
        where t.table_schema = 'public'
    ";

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&args.postgres_connection_string)
        .await
        .map_err(|e| connector::ConfigurationError::Other(e.into()))?;

    let query = sqlx::query(statement_string);

    let row = query
        .fetch_one(&pool)
        .await
        .map_err(|e| connector::ConfigurationError::Other(e.into()))?;

    let tables: query_engine::metadata::TablesInfo = serde_json::from_value(row.get(0))
        .map_err(|e| connector::ConfigurationError::Other(e.into()))?;

    Ok(DeploymentConfiguration {
        version: 1,
        postgres_database_url: args.postgres_connection_string.clone(),
        tables,
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
