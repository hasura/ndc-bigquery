use ndc_hub::connector;
use ndc_hub::models;

use super::configuration;
use query_engine::phases;
use tracing::{info_span, Instrument};

/// Execute a query
///
/// This function implements the [query endpoint](https://hasura.github.io/ndc-spec/specification/queries/index.html)
/// from the NDC specification.
pub async fn query(
    configuration: &configuration::DeploymentConfiguration,
    state: &configuration::State,
    query_request: models::QueryRequest,
) -> Result<models::QueryResponse, connector::QueryError> {
    tracing::info!("{}", serde_json::to_string(&query_request).unwrap());
    tracing::info!("{:?}", query_request);

    // Compile the query.
    let plan = async {
        match phases::translation::query::translate(&configuration.metadata, query_request) {
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
        }
    }
    .instrument(info_span!("Plan query"))
    .await?;

    // Execute the query.
    let result = phases::execution::execute(&state.pool, plan)
        .instrument(info_span!("Execute query"))
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
