//! Implement the `/query` endpoint to run a query against postgres.
//! See the spec for further details:
//! https://hasura.github.io/ndc-spec/specification/queries/index.html

use tracing::{info_span, Instrument};

use ndc_sdk::connector;
use ndc_sdk::models;
use query_engine::phases;

use super::configuration;

/// Execute a query
///
/// This function implements the [query endpoint](https://hasura.github.io/ndc-spec/specification/queries/index.html)
/// from the NDC specification.
pub async fn query(
    configuration: &configuration::Configuration,
    state: &configuration::State,
    query_request: models::QueryRequest,
) -> Result<models::QueryResponse, connector::QueryError> {
    // See https://docs.rs/tracing/0.1.29/tracing/span/struct.Span.html#in-asynchronous-code
    async move {
        tracing::info!(
            query_request_json = serde_json::to_string(&query_request).unwrap(),
            query_request = ?query_request
        );

        // Compile the query.
        let plan = match phases::translation::query::translate(
            &configuration.config.metadata,
            query_request,
        ) {
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

        state.metrics.record_successful_query();

        Ok(result)
    }
    .instrument(info_span!("/query"))
    .await
}
