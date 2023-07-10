/// Execute an execution plan against the database.
use serde_json;
use std::collections::HashMap;

use sqlx;
use sqlx::postgres::types::Oid;
use sqlx::postgres::PgTypeInfo;
use sqlx::Column;
use sqlx::Row;

use super::translation;
use gdc_client::models;

pub async fn execute(
    pool: sqlx::PgPool,
    plan: translation::ExecutionPlan,
) -> Result<models::QueryResponse, sqlx::Error> {
    let query = plan.query();

    tracing::info!("Generated SQL: {}", query.sql);

    // fetch from the database
    let rows: Vec<sqlx::postgres::PgRow> = sqlx::query(query.sql.as_str()).fetch_all(&pool).await?;

    // convert to a vector of hashmap of results
    let mut results: Vec<HashMap<String, models::RowFieldValue>> = vec![];
    for row in rows.into_iter() {
        //tracing::info!("{:?}", row.columns());

        let mut row_results: HashMap<String, models::RowFieldValue> = HashMap::from([]);
        for col in row.columns() {
            let value = decode_value(&row, col)?;

            row_results.insert(
                col.name().to_string(),
                models::RowFieldValue::Column { value },
            );
        }
        results.push(row_results);
    }

    //tracing::info!("{:?}", results);

    // return results
    Ok(models::QueryResponse(vec![models::RowSet {
        aggregates: None,
        rows: Some(results),
    }]))
}

/// Decode values based on the column Oid.
/// Here's our matching list:
/// https://github.com/hasura/graphql-engine/blob/master/server/lib/pg-client/src/Database/PG/Query/PTI.hs
fn decode_value(
    row: &sqlx::postgres::PgRow,
    col: &sqlx::postgres::PgColumn,
) -> Result<serde_json::Value, sqlx::Error> {
    // See possible errors: https://docs.rs/sqlx/latest/sqlx/trait.Row.html#method.try_get
    let typ: PgTypeInfo = col.type_info().clone();
    match typ.kind() {
        sqlx::postgres::PgTypeKind::Simple => match typ.oid() {
            // bool
            Some(Oid(16)) => Ok(row
                .get::<Option<bool>, usize>(col.ordinal())
                .map_or(serde_json::Value::Null, serde_json::Value::Bool)),

            // int2
            Some(Oid(21)) => Ok(row
                .get::<Option<i16>, usize>(col.ordinal())
                .map_or(serde_json::Value::Null, |v| {
                    serde_json::Value::Number(i16::into(v))
                })),

            // int4
            Some(Oid(23)) => Ok(row
                .get::<Option<i32>, usize>(col.ordinal())
                .map_or(serde_json::Value::Null, |v| {
                    serde_json::Value::Number(i32::into(v))
                })),

            // int8
            Some(Oid(20)) => Ok(row
                .get::<Option<i64>, usize>(col.ordinal())
                .map_or(serde_json::Value::Null, |v| {
                    serde_json::Value::Number(i64::into(v))
                })),

            // text or varchar
            Some(Oid(25 | 1043)) => Ok(row
                .get::<Option<String>, usize>(col.ordinal())
                .map_or(serde_json::Value::Null, serde_json::Value::String)),

            // json or jsonb
            Some(Oid(114 | 3802)) => {
                let value = row.get(col.ordinal());
                Ok(value)
            }
            _ => Err(sqlx::Error::TypeNotFound {
                type_name: "Unsupported type".to_string(),
            }),
        },
        _ => Err(sqlx::Error::TypeNotFound {
            type_name: "Unsupported type".to_string(),
        }),
    }
}
