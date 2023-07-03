/// Execute an execution plan against the database.
use serde_json::Value;
use sqlx;
use sqlx::Column;
use sqlx::Row;
use std::collections::HashMap;

use super::translation;
use gdc_client::models;

pub async fn execute(
    pool: sqlx::PgPool,
    plan: translation::ExecutionPlan,
) -> Result<models::QueryResponse, sqlx::Error> {
    let query = plan.query();

    println!("{}", query.sql);

    // fetch from the database
    let rows: Vec<sqlx::postgres::PgRow> = sqlx::query(query.sql.as_str()).fetch_all(&pool).await?;

    // convert to a vector of hashmap of results
    let mut results: Vec<HashMap<String, models::RowFieldValue>> = vec![];
    for row in rows.into_iter() {
        //println!("{:?}", row.columns());

        let mut row_results: HashMap<String, models::RowFieldValue> = HashMap::from([]);
        for col in row.columns() {
            let value: String = row.get(col.ordinal());
            row_results.insert(
                col.name().to_string(),
                models::RowFieldValue::Column {
                    value: Value::String(value),
                },
            );
        }
        results.push(row_results);
    }

    //println!("{:?}", results);

    // return results
    Ok(models::QueryResponse(vec![models::RowSet {
        aggregates: None,
        rows: Some(results),
    }]))
}
