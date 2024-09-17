//! Execute an execution plan against the database.

use crate::error::{Error, QueryError};
use crate::metrics;
use bytes::{BufMut, Bytes, BytesMut};
use gcp_bigquery_client::model::query_request::QueryRequest;
use gcp_bigquery_client::model::{query_parameter, query_parameter_type, query_parameter_value};
use query_engine_sql::sql::string::Param;
use serde_json::{self, to_string, Value};
use sqlformat;
use sqlx;
use sqlx::Row;
use std::collections::BTreeMap;
use tracing::{info_span, Instrument};

use ndc_models as models;

use query_engine_sql::sql;

/// Execute a query against postgres.
pub async fn execute(
    bigquery_client: &gcp_bigquery_client::Client,
    metrics: &metrics::Metrics,
    project_id: &String,
    plan: sql::execution_plan::ExecutionPlan<sql::execution_plan::Query>,
) -> Result<Bytes, Error> {
    // let query_timer = metrics.time_query_execution();
    // let query = plan.query;
    // print!("{:?}", query.params);

    // tracing::info!(
    //     generated_sql = query.sql,
    //     params = ?&query.params,
    //     variables = ?&plan.variables,
    // );

    let mut buffer = BytesMut::new();

    // run the query on each set of variables. The result is a vector of rows each
    // element in the vector is the result of running the query on one set of variables.
    let rows = match plan.query.variables {
        None => {
            // TODO: need to parse this from service account key or allow user to provide it
            // TODO(PY)
            // let project_id = "hasura-development";

            // let mut inner_rows = vec![];

            let mut query_request = QueryRequest::new(plan.query.query_sql().sql);

            // smash query.params in here pls
            query_request.query_parameters = Some(
                plan.query
                    .query_sql()
                    .params
                    // .params
                    .iter()
                    .enumerate()
                    .map(|(i, param)| match param {
                        Param::String(str) => {
                            let value = query_parameter_value::QueryParameterValue {
                                array_values: None,
                                struct_values: None,
                                value: Some(str.to_string()),
                            };
                            let value_type = query_parameter_type::QueryParameterType {
                                array_type: None,
                                struct_types: None,
                                r#type: "STRING".to_string(),
                            };
                            query_parameter::QueryParameter {
                                name: Some(format!("param{}", i + 1)),
                                parameter_type: Some(value_type),
                                parameter_value: Some(value),
                            }
                        }
                        Param::Variable(_var) => todo!("Variables not implemented yet"), // error that `Err(Error::Query(QueryError::VariableNotFound(var.to_string())))`
                        Param::Value(_value) => todo!("Values not implemented yet"),     // todo(PY)
                    })
                    .collect(),
            );
            dbg!(&query_request);

            // Query
            let mut rs = bigquery_client
                .job()
                .query(project_id.as_str(), query_request)
                .await
                .unwrap();

            while rs.next_row() {
                dbg!("result set of row: ", &rs);
                let this_row = rs.get_string(0).unwrap().unwrap(); // we should only have one row called 'universe'
                dbg!("this row: ", &this_row);
                let foo: Value = serde_json::from_str(&this_row).unwrap();
                dbg!("foo: ", &foo);
                let bar = Value::Array(vec![foo]);
                dbg!("bar: ", &bar);
                let baz = to_string(&bar).unwrap();
                dbg!("baz: ", &baz);
                // let bar: u8 = this_row.as_bytes()[0];
                // dbg!("bar: ", &bar);
                // let foo = vec![this_row];
                // let json_value = serde_json::from_str(&this_row).unwrap();
                let b: Bytes = Bytes::from(baz);
                // let b: Bytes = Bytes::from(to_string(&foo).unwrap());
                dbg!("b: ", &b);
                buffer.put(b);
                // let this_row = rs.get_json_value(0).unwrap(); // we should only have one row called 'universe'
                //                                                    // let json_value = serde_json::from_str(&this_row).unwrap();
                // let json_string = serde_json::to_string(&this_row).unwrap();
                // let b: Bytes = Bytes::from(json_string);
                // buffer.put(b);
                // inner_rows.push(json_value);
            }
            // let b: Bytes = Bytes::from(serde_json::to_string(&inner_rows).unwrap());
            // inner_rows
        }
        Some(_variable_sets) => {
            todo!("foreach/variables not implemented in query engine / execution")
        }
    };

    Ok(buffer.freeze())

    // tracing::info!(rows_result = ?rows);
    // query_timer.complete_with(rows)

    // Make a response from rows.
    // let response = async { rows_to_response(rows) }
    //     .instrument(info_span!("Create response"))
    //     .await;

    // tracing::info!(query_response = serde_json::to_string(&response).unwrap());

    // Ok(response)
}

/// Take the postgres results and return them as a QueryResponse.
fn rows_to_response(results: Vec<serde_json::Value>) -> models::QueryResponse {
    let rowsets = results
        .into_iter()
        .map(|raw_rowset| serde_json::from_value(raw_rowset).unwrap())
        .collect();

    models::QueryResponse(rowsets)
}

// TODO(PY): add explain for BigQuery

// /// Convert a query to an EXPLAIN query and execute it against postgres.
// pub async fn explain(
//     pool: &sqlx::PgPool,
//     plan: sql::execution_plan::ExecutionPlan<sql::execution_plan::Query>,
// ) -> Result<(String, String), Error> {
//     let query = plan.query.explain_query_sql();

//     tracing::info!(
//         generated_sql = query.sql,
//         params = ?&query.params,
//         variables = ?&plan.query.variables,
//     );

//     let empty_map = BTreeMap::new();
//     let sqlx_query = match &plan.query.variables {
//         None => build_query_with_params(&query, &empty_map).await?,
//         // When we get an explain with multiple variable sets,
//         // we choose the first one and return the plan for it,
//         // as returning multiple plans isn't really supported.
//         Some(variable_sets) => match variable_sets.get(0) {
//             None => build_query_with_params(&query, &empty_map).await?,
//             Some(vars) => build_query_with_params(&query, vars).await?,
//         },
//     };

//     // run and fetch from the database
//     let rows: Vec<sqlx::postgres::PgRow> = sqlx_query.fetch_all(pool).await?;

//     let mut results: Vec<String> = vec![];
//     for row in rows.into_iter() {
//         match row.get(0) {
//             None => {}
//             Some(col) => {
//                 results.push(col);
//             }
//         }
//     }

//     let pretty = sqlformat::format(
//         &query.sql,
//         &sqlformat::QueryParams::None,
//         sqlformat::FormatOptions::default(),
//     );

//     Ok((pretty, results.join("\n")))
// }

/// Create a SQLx query based on our SQL query and bind our parameters and variables to it.
async fn build_query_with_params<'a>(
    query: &'a sql::string::SQL,
    variables: Option<&'a [BTreeMap<models::VariableName, serde_json::Value>]>,
) -> Result<sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments>, Error> {
    let sqlx_query = sqlx::query(query.sql.as_str());

    query
        .params
        .iter()
        .try_fold(sqlx_query, |sqlx_query, param| match param {
            sql::string::Param::String(s) => Ok(sqlx_query.bind(s)),
            sql::string::Param::Value(v) => Ok(sqlx_query.bind(v)),
            sql::string::Param::Variable(var)
                if var == sql::helpers::VARIABLES_OBJECT_PLACEHOLDER =>
            {
                match &variables {
                    None => Err(Error::Query(QueryError::VariableNotFound(var.to_string()))),
                    Some(variables) => {
                        let vars = variables_to_json(variables)?;
                        Ok(sqlx_query.bind(vars))
                    }
                }
            }
            sql::string::Param::Variable(var) => {
                Err(Error::Query(QueryError::VariableNotFound(var.to_string())))
            } // sql::string::Param::Variable(var) => match variables.get(var) {
              //     Some(value) => match value {
              //         serde_json::Value::String(s) => Ok(sqlx_query.bind(s)),
              //         serde_json::Value::Number(n) => Ok(sqlx_query.bind(n.as_f64())),
              //         serde_json::Value::Bool(b) => Ok(sqlx_query.bind(b)),
              //         // this is a problem - we don't know the type of the value!
              //         serde_json::Value::Null => Err(Error::Query(
              //             "null variable not currently supported".to_string(),
              //         )),
              //         serde_json::Value::Array(_array) => Err(Error::Query(
              //             "array variable not currently supported".to_string(),
              //         )),
              //         serde_json::Value::Object(_object) => Err(Error::Query(
              //             "object variable not currently supported".to_string(),
              //         )),
              //     },
              //     None => Err(Error::Query(format!("Variable not found '{}'", var))),
              // },
        })

    // Ok(sqlx_query)
}

/// build an array of variable set objects that will be passed as parameters to postgres.
fn variables_to_json(
    variables: &[BTreeMap<models::VariableName, serde_json::Value>],
) -> Result<serde_json::Value, Error> {
    Ok(serde_json::Value::Array(
        variables
            .iter()
            .enumerate()
            .map(|(i, varset)| {
                let mut row = serde_json::Map::new();

                row.insert(
                    sql::helpers::VARIABLE_ORDER_FIELD.to_string(),
                    serde_json::Value::Number(i.into()),
                );

                let variables_field = serde_json::Value::Object(
                    varset
                        .iter()
                        .map(|(argument, value)| (argument.to_string(), value.clone()))
                        .collect::<serde_json::Map<String, serde_json::Value>>(),
                );
                row.insert(sql::helpers::VARIABLES_FIELD.to_string(), variables_field);

                Ok(serde_json::Value::Object(row))
            })
            .collect::<Result<Vec<serde_json::Value>, Error>>()?,
    ))
}

// pub enum Error {
//     Query(String),
//     DB(sqlx::Error),
// }

// impl From<sqlx::Error> for Error {
//     fn from(err: sqlx::Error) -> Error {
//         Error::DB(err)
//     }
// }
