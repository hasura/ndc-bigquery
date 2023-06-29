/// Execute an execution plan against the database.
use serde_json::Value;
use sqlx;
use sqlx::Column;
use sqlx::Row;
use std::collections::HashMap;

use super::translation;
use crate::types::output;

pub async fn execute(
    pool: sqlx::PgPool,
    plan: translation::ExecutionPlan,
) -> Result<output::QueryResponse, sqlx::Error> {
    let query = plan.query();

    println!("{}", query.sql);

    // fetch from the database
    let rows: Vec<sqlx::postgres::PgRow> = sqlx::query(query.sql.as_str()).fetch_all(&pool).await?;

    // convert to a vector of hashmap of results
    let mut results: Vec<HashMap<String, output::RowFieldValue>> = vec![];
    for row in rows.into_iter() {
        println!("{:?}", row.columns());

        let mut row_results: HashMap<String, output::RowFieldValue> = HashMap::from([]);
        for col in row.columns() {
            let value: i32 = row.get(col.ordinal());
            row_results.insert(
                col.name().to_string(),
                output::RowFieldValue::Column(Value::Number(value.into())),
            );
        }
        results.push(row_results);
    }

    // return results
    Ok(output::QueryResponse(vec![output::RowSet {
        rows: Some(results),
    }]))
}
