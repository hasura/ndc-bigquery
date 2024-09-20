//! Health check endpoint for the connector.

use ndc_sdk::connector::ErrorResponse;

use gcp_bigquery_client::model::query_request::QueryRequest;

/// Check the health of the connector.
///
/// For example, this function should check that the connector
/// is able to reach its data source over the network.
pub async fn health_check(
    bigquery_client: &gcp_bigquery_client::Client,
) -> Result<(), ErrorResponse> {
    // TODO: need to parse this from service account key or allow user to provide it
    let project_id = "hasura-development";

    // Query
    let mut rs = bigquery_client
        .job()
        .query(
            project_id,
            QueryRequest::new("SELECT 1 as count".to_string()),
        )
        .await
        .unwrap();

    // silly check
    let mut count = 0;

    while rs.next_row() {
        count += 1;
    }

    assert_eq!(count, 1);

    Ok(())
}
