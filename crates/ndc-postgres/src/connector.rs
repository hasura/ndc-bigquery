/// Define a Connector trait for Postgres, use the relevant types
/// for configuration and state as defined in `super::configuration`,
/// and define the route handling for each route.
use super::configuration;
use super::metrics;
use ndc_hub::connector;
use query_engine::phases;

use async_trait::async_trait;
use ndc_client::models;
use std::collections::BTreeMap;

#[derive(Clone, Default)]
pub struct Postgres {}

#[async_trait]
impl connector::Connector for Postgres {
    /// Arguments for configuration?
    type ConfigureArgs = configuration::ConfigureArgs;
    /// RawConfiguration is what the user specifies as JSON
    type RawConfiguration = configuration::DeploymentConfiguration;
    /// Configuration is the validated version of that
    /// which gets stored as binary by us (when we host the agent).
    type Configuration = configuration::DeploymentConfiguration;
    /// State is the in memory representation of a loaded Configuration,
    /// e.g. any connection pool, other handles etc. which might not be serializable
    type State = configuration::State;

    /// Configure a configuration maybe?
    async fn configure(
        args: &Self::ConfigureArgs,
    ) -> Result<configuration::DeploymentConfiguration, connector::ConfigurationError> {
        configuration::configure(args).await
    }

    // update metrics in time for `/metrics` call
    fn fetch_metrics(
        _configuration: &configuration::DeploymentConfiguration,
        state: &configuration::State,
    ) -> Result<(), connector::FetchMetricsError> {
        metrics::update_pool_metrics(&state.pool, &state.metrics);
        Ok(())
    }

    /// Validate the config.
    async fn validate_raw_configuration(
        configuration: &Self::RawConfiguration,
    ) -> Result<Self::Configuration, connector::ConfigurationError> {
        configuration::validate_raw_configuration(configuration).await
    }

    /// Initialize the connector state. Which is pretty much just initializing the pool.
    async fn try_init_state(
        configuration: &Self::Configuration,
        metrics: &mut prometheus::Registry,
    ) -> Result<Self::State, connector::InitializationError> {
        configuration::create_state(configuration, metrics).await
    }

    /// @todo: dummy for now
    async fn health_check(
        _configuration: &Self::Configuration,
        _state: &Self::State,
    ) -> Result<(), connector::HealthError> {
        Ok(())
    }

    /// Return the capabilities of the connector.
    async fn get_capabilities() -> models::CapabilitiesResponse {
        let empty = serde_json::to_value(()).unwrap();
        models::CapabilitiesResponse {
            versions: "^0.0.0".into(),
            capabilities: models::Capabilities {
                explain: Some(empty.clone()),
                query: Some(models::QueryCapabilities {
                    foreach: Some(empty),
                    order_by_aggregate: None,
                    relation_comparisons: None,
                }),
                relationships: None,
                mutations: None,
            },
        }
    }

    /// Explain a query against postgres and return the query sql and plan.
    async fn explain(
        configuration: &Self::Configuration,
        state: &Self::State,
        query_request: models::QueryRequest,
    ) -> Result<models::ExplainResponse, connector::ExplainError> {
        tracing::info!("{}", serde_json::to_string(&query_request).unwrap());
        tracing::info!("{:?}", query_request);

        // Compile the query.
        let plan = match phases::translation::query::translate(&configuration.tables, query_request)
        {
            Ok(plan) => Ok(plan),
            Err(err) => {
                tracing::error!("{}", err);
                Err(connector::ExplainError::Other(err.to_string().into()))
            }
        }?;

        // Execute an explain query.
        let (query, plan) = phases::execution::explain(&state.pool, plan)
            .await
            .map_err(|err| match err {
                phases::execution::Error::Query(err) => {
                    tracing::error!("{}", err);
                    connector::ExplainError::Other(err.into())
                }
                phases::execution::Error::DB(err) => {
                    tracing::error!("{}", err);
                    connector::ExplainError::Other(err.to_string().into())
                }
            })?;

        // assuming explain succeeded, increment counter
        state.metrics.explain_total.inc();

        let details =
            BTreeMap::from_iter([("SQL Query".into(), query), ("Execution Plan".into(), plan)]);

        let response = models::ExplainResponse { details };

        Ok(response)
    }

    /// Compile the query request to SQL, run it against postgres, and return the results.
    async fn query(
        configuration: &Self::Configuration,
        state: &Self::State,
        query_request: models::QueryRequest,
    ) -> Result<models::QueryResponse, connector::QueryError> {
        tracing::info!("{}", serde_json::to_string(&query_request).unwrap());
        tracing::info!("{:?}", query_request);

        // Compile the query.
        let plan = match phases::translation::query::translate(&configuration.tables, query_request)
        {
            Ok(plan) => Ok(plan),
            Err(err) => {
                tracing::error!("{}", err);
                match err {
                    phases::translation::query::error::Error::NotSupported(_) => {
                        Err(connector::QueryError::UnsupportedOperation(err.to_string()))
                    }
                    _ => Err(connector::QueryError::InvalidRequest(err.to_string())),
                }
            }
        }?;

        // Execute the query.
        let result = phases::execution::execute(&state.pool, plan)
            .await
            .map_err(|err| match err {
                phases::execution::Error::Query(err) => {
                    tracing::error!("{}", err);
                    connector::QueryError::Other(err.into())
                }
                phases::execution::Error::DB(err) => {
                    tracing::error!("{}", err);
                    connector::QueryError::Other(err.to_string().into())
                }
            })?;

        // assuming query succeeded, increment counter
        state.metrics.query_total.inc();

        Ok(result)
    }

    /// @todo
    async fn get_schema(
        configuration: &Self::Configuration,
    ) -> Result<models::SchemaResponse, connector::SchemaError> {
        let scalar_types = BTreeMap::from_iter([(
            "any".into(),
            ndc_client::models::ScalarType {
                aggregate_functions: BTreeMap::new(),
                comparison_operators: BTreeMap::new(),
                update_operators: BTreeMap::new(),
            },
        )]);

        let query_engine::metadata::TablesInfo(tablesinfo) = &configuration.tables;
        let collections = tablesinfo
            .iter()
            .map(|(table_name, _table)| ndc_client::models::CollectionInfo {
                name: table_name.clone(),
                description: None,
                arguments: BTreeMap::new(),
                collection_type: table_name.clone(),
                insertable_columns: None,
                updatable_columns: None,
                deletable: false,
                uniqueness_constraints: BTreeMap::new(),
                foreign_keys: BTreeMap::new(),
            })
            .collect();

        let object_types = BTreeMap::from_iter(tablesinfo.iter().map(|(table_name, table)| {
            let object_type = models::ObjectType {
                description: None,
                fields: BTreeMap::from_iter(table.columns.values().map(|column| {
                    (
                        column.name.clone(),
                        models::ObjectField {
                            arguments: BTreeMap::new(),
                            description: None,
                            r#type: models::Type::Named { name: "any".into() },
                        },
                    )
                })),
            };
            (table_name.clone(), object_type)
        }));

        Ok(models::SchemaResponse {
            collections,
            procedures: vec![],
            functions: vec![],
            object_types,
            scalar_types,
        })
    }

    /// @todo: dummy for now
    async fn mutation(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::MutationRequest,
    ) -> Result<models::MutationResponse, connector::MutationError> {
        todo!()
    }
}
