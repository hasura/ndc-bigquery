/// Define a Connector trait for Postgres, use the relevant types
/// for configuration and state as defined in `super::configuration`,
/// and define the route handling for each route.
use super::configuration;
use ndc_hub::connector;
use query_engine::phases;

use async_trait::async_trait;
use ndc_client::models;
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct Postgres {}

#[async_trait]
impl connector::Connector for Postgres {
    /// RawConfiguration is what the user specifies as JSON
    type RawConfiguration = configuration::Configuration;
    /// Configuration is the validated version of that
    /// which gets stored as binary by us (when we host the agent).
    type Configuration = configuration::Configuration;
    /// State is the in memory representation of a loaded Configuration,
    /// e.g. any connection pool, other handles etc. which might not be serializable
    type State = configuration::State;

    /// Validate the config.
    async fn validate_raw_configuration(
        configuration: &Self::RawConfiguration,
    ) -> Result<Self::Configuration, connector::ConfigurationError> {
        configuration::validate_raw_configuration(configuration).await
    }

    /// Initialize the connector state. Which is pretty much just initializing the pool.
    async fn try_init_state(
        configuration: &Self::Configuration,
        _metrics: &mut prometheus::Registry,
    ) -> Result<Self::State, connector::InitializationError> {
        configuration::create_state(configuration).await
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
        let plan = match phases::translation::translate(&configuration.tables, query_request) {
            Ok(plan) => Ok(plan),
            Err(err) => {
                tracing::error!("{}", err);
                Err(connector::ExplainError::Other(err.to_string().into()))
            }
        }?;

        // Execute an explain query.
        let (query, lines) = match phases::execution::explain(&state.pool, plan).await {
            Ok(plan) => Ok(plan),
            Err(err) => Err(match err {
                phases::execution::Error::Query(err) => {
                    tracing::error!("{}", err);
                    connector::ExplainError::Other(err.into())
                }
                phases::execution::Error::DB(err) => {
                    tracing::error!("{}", err);
                    connector::ExplainError::Other(err.to_string().into())
                }
            }),
        }?;

        let response = models::ExplainResponse { lines, query };

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
        let plan = match phases::translation::translate(&configuration.tables, query_request) {
            Ok(plan) => Ok(plan),
            Err(err) => {
                tracing::error!("{}", err);
                match err {
                    phases::translation::Error::NotSupported(_) => {
                        Err(connector::QueryError::UnsupportedOperation(err.to_string()))
                    }
                    _ => Err(connector::QueryError::InvalidRequest(err.to_string())),
                }
            }
        }?;

        // Execute the query.
        let result = match phases::execution::execute(&state.pool, plan).await {
            Ok(plan) => Ok(plan),
            Err(err) => Err(match err {
                phases::execution::Error::Query(err) => {
                    tracing::error!("{}", err);
                    connector::QueryError::Other(err.into())
                }
                phases::execution::Error::DB(err) => {
                    tracing::error!("{}", err);
                    connector::QueryError::Other(err.to_string().into())
                }
            }),
        }?;

        Ok(result)
    }

    /// @todo: dummy for now
    async fn get_schema(
        _configuration: &Self::Configuration,
    ) -> Result<models::SchemaResponse, connector::SchemaError> {
        Ok(models::SchemaResponse {
            commands: vec![],
            tables: vec![],
            object_types: HashMap::new(),
            scalar_types: HashMap::new(),
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
