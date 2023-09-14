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
pub async fn query<'a>(
    configuration: &configuration::InternalConfiguration<'a>,
    state: &configuration::State,
    query_request: models::QueryRequest,
) -> Result<models::QueryResponse, connector::QueryError> {
    // See https://docs.rs/tracing/0.1.29/tracing/span/struct.Span.html#in-asynchronous-code
    async move {
        tracing::info!(
            query_request_json = serde_json::to_string(&query_request).unwrap(),
            query_request = ?query_request
        );

        let plan = plan_query(configuration, state, query_request)?;
        let result = execute_query(state, plan).await?;
        state.metrics.record_successful_query();
        Ok(result)
    }
    .instrument(info_span!("/query"))
    .await
}

fn plan_query(
    configuration: &configuration::InternalConfiguration,
    state: &configuration::State,
    query_request: models::QueryRequest,
) -> Result<phases::translation::sql::execution_plan::ExecutionPlan, connector::QueryError> {
    let timer = state.metrics.time_query_plan();
    let result = phases::translation::query::translate(configuration.metadata, query_request)
        .map_err(|err| {
            tracing::error!("{}", err);
            match err {
                phases::translation::query::error::Error::NotSupported(_) => {
                    connector::QueryError::UnsupportedOperation(err.to_string())
                }
                _ => connector::QueryError::InvalidRequest(err.to_string()),
            }
        });
    timer.complete_with(result)
}

async fn execute_query(
    state: &configuration::State,
    plan: phases::translation::sql::execution_plan::ExecutionPlan,
) -> Result<models::QueryResponse, connector::QueryError> {
    let timer = state.metrics.time_query_execution();
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
        });
    timer.complete_with(result)
}
