//! Translate an incoming `QueryRequest`.

pub mod aggregates;
pub mod error;
pub mod filtering;
pub mod helpers;
pub mod relationships;
pub mod root;
pub mod sorting;

use ndc_hub::models;

use crate::metadata;
use crate::phases::translation::query::helpers::Env;
use crate::phases::translation::sql;
use error::Error;

/// Translate the incoming QueryRequest to an ExecutionPlan (SQL) to be run against the database.
pub fn translate(
    metadata: &metadata::Metadata,
    query_request: models::QueryRequest,
) -> Result<sql::execution_plan::ExecutionPlan, Error> {
    let select_set = translate_query(
        &Env {
            metadata: metadata.clone(),
            relationships: query_request.collection_relationships,
        },
        query_request.collection.clone(),
        query_request.query,
    )?;

    // form a single JSON item shaped `{ rows: [], aggregates: {} }`
    // that matches the models::RowSet type
    let json_select = sql::helpers::select_rowset(
        sql::helpers::make_column_alias("universe".to_string()),
        sql::helpers::make_table_alias("universe".to_string()),
        sql::helpers::make_table_alias("rows".to_string()),
        sql::helpers::make_column_alias("rows".to_string()),
        sql::helpers::make_table_alias("aggregates".to_string()),
        sql::helpers::make_column_alias("aggregates".to_string()),
        select_set,
    );

    // log and return
    tracing::info!("SQL AST: {:?}", json_select);
    Ok(sql::execution_plan::simple_exec_plan(
        query_request.variables,
        query_request.collection,
        json_select,
    ))
}

/// Translate a query to sql ast.
/// We return a SELECT for the 'rows' field and a SELECT for the 'aggregates' field.
pub fn translate_query(
    env: &Env,
    table_name: String,
    query: models::Query,
) -> Result<sql::helpers::SelectSet, Error> {
    // Error::NoFields becomes Ok(None)
    // everything stays Err
    let map_no_fields_error_to_none = |err| match err {
        Error::NoFields => Ok(None),
        other_error => Err(other_error),
    };

    // wrap valid result in Some
    let wrap_ok = |a| Ok(Some(a));

    // translate rows query. if there are no fields, make this a None
    let row_select: Option<sql::ast::Select> = root::translate_rows_query(env, &table_name, &query)
        .map_or_else(map_no_fields_error_to_none, wrap_ok)?;

    // translate aggregate select. if there are no fields, make this a None
    let aggregate_select: Option<sql::ast::Select> =
        root::translate_aggregate_query(env, table_name, &query)
            .map_or_else(map_no_fields_error_to_none, wrap_ok)?;

    match (row_select, aggregate_select) {
        (Some(rows), None) => Ok(sql::helpers::SelectSet::Rows(rows)),
        (None, Some(aggregates)) => Ok(sql::helpers::SelectSet::Aggregates(aggregates)),
        (Some(rows), Some(aggregates)) => {
            Ok(sql::helpers::SelectSet::RowsAndAggregates(rows, aggregates))
        }
        _ => Err(Error::NoFields),
    }
}
