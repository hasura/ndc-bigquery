/// Execute an execution plan against the database.
use serde_json;

use sqlx;
use sqlx::Row;
use std::collections::HashMap;

use super::response_hack;
use super::translation::{sql_string, ExecutionPlan};
use gdc_client::models;

pub async fn execute(
    pool: sqlx::PgPool,
    plan: ExecutionPlan,
) -> Result<models::QueryResponse, Error> {
    let query = plan.query();

    tracing::info!(
        "\nGenerated SQL: {}\nWith params: {:?}\nAnd variables: {:?}",
        query.sql,
        &query.params,
        &plan.variables,
    );

    // run the query on each set of variables. The result is a vector of rows each
    // element in the vector is the result of running the query on one set of variables.
    let rows: Option<Vec<serde_json::Value>> = match plan.variables {
        None => {
            let empty_hashmap = HashMap::new();
            let rows = execute_query(&pool, &query, &empty_hashmap).await?;
            Some(vec![rows])
        }
        Some(variable_sets) => {
            let mut sets_of_rows = vec![];
            for vars in &variable_sets {
                let rows = execute_query(&pool, &query, vars).await?;
                sets_of_rows.push(rows);
            }
            Some(sets_of_rows)
        }
    };

    // tracing::info!("Database rows result: {:?}", rows);

    // Hack a response from the query results. See the 'response_hack' for more details.
    let response = response_hack::rows_to_response(rows);

    // tracing::info!(
    //     "Query response: {}",
    //     serde_json::to_string(&response).unwrap()
    // );

    Ok(response)
}

/// Execute the query on one set of variables.
async fn execute_query(
    pool: &sqlx::PgPool,
    query: &sql_string::SQL,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, Error> {
    // build query
    let sqlx_query = build_query_with_params(query, variables).await?;

    // run and fetch from the database
    let rows = sqlx_query
        .map(|row: sqlx::postgres::PgRow| row.get(0))
        .fetch_one(pool)
        .await?;

    Ok(rows)
}

/// Create a SQLx query based on our SQL query and bind our parameters and variables to it.
async fn build_query_with_params<'a>(
    query: &'a sql_string::SQL,
    variables: &'a HashMap<String, serde_json::Value>,
) -> Result<sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments>, Error> {
    let sqlx_query = sqlx::query(query.sql.as_str());

    let sqlx_query = query
        .params
        .iter()
        .try_fold(sqlx_query, |sqlx_query, param| match param {
            sql_string::Param::String(s) => Ok(sqlx_query.bind(s)),
            sql_string::Param::Variable(var) => match variables.get(var) {
                Some(value) => match value {
                    serde_json::Value::String(s) => Ok(sqlx_query.bind(s)),
                    serde_json::Value::Number(n) => Ok(sqlx_query.bind(n.as_f64())),
                    serde_json::Value::Bool(b) => Ok(sqlx_query.bind(b)),
                    // this is a problem - we don't know the type of the value!
                    serde_json::Value::Null => Err(Error::Query(
                        "null variable not currently supported".to_string(),
                    )),
                    serde_json::Value::Array(_array) => Err(Error::Query(
                        "array variable not currently supported".to_string(),
                    )),
                    serde_json::Value::Object(_object) => Err(Error::Query(
                        "object variable not currently supported".to_string(),
                    )),
                },
                None => Err(Error::Query(format!("Variable not found '{}'", var))),
            },
        })?;

    Ok(sqlx_query)
}

pub enum Error {
    Query(String),
    DB(sqlx::Error),
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Error {
        Error::DB(err)
    }
}
