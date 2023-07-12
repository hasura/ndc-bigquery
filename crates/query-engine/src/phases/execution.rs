/// Execute an execution plan against the database.
use serde_json;

use sqlx;
use sqlx::Row;

use super::response_hack;
use super::translation;
use gdc_client::models;

pub async fn execute(
    pool: sqlx::PgPool,
    plan: translation::ExecutionPlan,
) -> Result<models::QueryResponse, sqlx::Error> {
    let query = plan.query();

    tracing::info!(
        "Generated SQL: {} With params {:?}",
        query.sql,
        &query.params
    );

    let sqlx_query = sqlx::query(query.sql.as_str());

    let sqlx_query = query
        .params
        .into_iter()
        .fold(sqlx_query, |sqlx_query, param| match param {
            translation::sql_string::Param::String(s) => sqlx_query.bind(s),
        });

    // fetch from the database
    let rows: serde_json::Value = sqlx_query
        .map(|row: sqlx::postgres::PgRow| row.get(0))
        .fetch_one(&pool)
        .await?;

    // tracing::info!("Database rows result: {:?}", rows);

    // Hack a response from the query results. See the 'response_hack' for more details.
    let response = response_hack::rows_to_response(rows);

    // tracing::info!("Query response: {}", serde_json::to_string(&response).unwrap());

    Ok(response)
}
