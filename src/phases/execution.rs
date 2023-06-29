/// Execute an execution plan against the database.
use serde_json::Value;
use std::collections::HashMap;

use super::translation;
use crate::types::output;
use sqlx;

pub async fn execute(
    pool: sqlx::PgPool,
    plan: translation::ExecutionPlan,
) -> Result<output::QueryResponse, sqlx::Error> {
    let query = plan.query();
    println!("{}", query.sql);
    let rows = sqlx::query(query.sql.as_str())
        // .bind(sql.params)
        //.map(|row: sqlx::postgres::PgRow| row.len())
        .fetch_all(&pool)
        .await?;

    //println!("{:?}", values);
    Ok(output::QueryResponse(vec![output::RowSet {
        rows: Some(vec![HashMap::from([(
            "x".to_string(),
            output::RowFieldValue::Column {
                value: Value::String("hi".to_string()),
            },
        )])]),
    }]))
}
