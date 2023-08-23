use std::collections::BTreeMap;

use ndc_hub::connector;
use ndc_hub::models;

use super::configuration;
use query_engine::phases;
use tracing::{info_span, Instrument};

/// Explain a query by creating an execution plan
///
/// This function implements the [explain endpoint](https://hasura.github.io/ndc-spec/specification/explain.html)
/// from the NDC specification.
pub async fn explain(
    configuration: &configuration::DeploymentConfiguration,
    state: &configuration::State,
    query_request: models::QueryRequest,
) -> Result<models::ExplainResponse, connector::ExplainError> {
    tracing::info!("{}", serde_json::to_string(&query_request).unwrap());
    tracing::info!("{:?}", query_request);

    // Compile the query.
    let plan = async {
        match phases::translation::query::translate(&configuration.metadata, query_request) {
            Ok(plan) => Ok(plan),
            Err(err) => {
                tracing::error!("{}", err);
                Err(connector::ExplainError::Other(err.to_string().into()))
            }
        }
    }
    .instrument(info_span!("Plan query"))
    .await?;

    // Execute an explain query.
    let (query, plan) = phases::execution::explain(&state.pool, plan)
        .instrument(info_span!("Explain query"))
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
