//! Implement the `/explain` endpoint to return a query execution plan.
//! See the spec for further details:
//! https://hasura.github.io/ndc-spec/specification/explain.html

use std::collections::BTreeMap;

use tracing::{info_span, Instrument};

use ndc_sdk::connector;
use ndc_sdk::models;
use query_engine::phases;

use super::configuration;

/// Explain a query by creating an execution plan
///
/// This function implements the [explain endpoint](https://hasura.github.io/ndc-spec/specification/explain.html)
/// from the NDC specification.
pub async fn explain(
    configuration: &configuration::DeploymentConfiguration,
    state: &configuration::State,
    query_request: models::QueryRequest,
) -> Result<models::ExplainResponse, connector::ExplainError> {
    async move {
        tracing::info!(
            query_request_json = serde_json::to_string(&query_request).unwrap(),
            query_request = ?query_request
        );

        // Compile the query.
        let plan =
            match phases::translation::query::translate(&configuration.metadata, query_request) {
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
    .instrument(info_span!("/explain"))
    .await
}
