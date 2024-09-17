//! Execute an execution plan against the database.

use crate::error::Error;
use crate::metrics;
use bytes::{BufMut, Bytes, BytesMut};
use gcp_bigquery_client::model::query_request::QueryRequest;
use gcp_bigquery_client::model::{query_parameter, query_parameter_type, query_parameter_value};
use query_engine_sql::sql::string::Param;
use serde_json::{self, to_string, Value};

use query_engine_sql::sql;

/// Execute a query against postgres.
pub async fn execute(
    bigquery_client: &gcp_bigquery_client::Client,
    _metrics: &metrics::Metrics,
    project_id: &str,
    plan: sql::execution_plan::ExecutionPlan<sql::execution_plan::Query>,
) -> Result<Bytes, Error> {
    let mut buffer = BytesMut::new();

    // run the query on each set of variables. The result is a vector of rows each
    // element in the vector is the result of running the query on one set of variables.
    match plan.query.variables {
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
                .query(project_id, query_request)
                .await
                .unwrap();

            while rs.next_row() {
                dbg!("result set of row: ", &rs);
                let this_row = rs.get_string(0).unwrap().unwrap(); // we should only have one row called 'universe'
                dbg!("this row: ", &this_row);
                let row_value: Value = serde_json::from_str(&this_row).unwrap();
                dbg!("row_value: ", &row_value);
                let row_value_array = Value::Array(vec![row_value]);
                dbg!("row_value_array: ", &row_value_array);
                let final_row = to_string(&row_value_array).unwrap();
                dbg!("final_row: ", &final_row);
                let b: Bytes = Bytes::from(final_row);
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
}
