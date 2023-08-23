//! Translate an incoming `QueryRequest`.

mod aggregates;
pub mod error;
mod filtering;
mod helpers;
mod native_queries;
mod relationships;
mod root;
mod sorting;

use ndc_hub::models;

use crate::metadata;
use crate::phases::translation::query::helpers::{Env, State, TableNameAndReference};
use crate::phases::translation::sql;
use error::Error;

/// Translate the incoming QueryRequest to an ExecutionPlan (SQL) to be run against the database.
pub fn translate(
    metadata: &metadata::Metadata,
    query_request: models::QueryRequest,
) -> Result<sql::execution_plan::ExecutionPlan, Error> {
    let env = Env::new(metadata.clone(), query_request.collection_relationships);
    let mut state = State::new();
    let (current_table, from_clause) =
        root::make_from_clause_and_reference(&query_request.collection, &env, &mut state)?;

    let select_set = translate_query(
        &env,
        &mut state,
        &current_table,
        &from_clause,
        query_request.query,
    )?;

    // form a single JSON item shaped `{ rows: [], aggregates: {} }`
    // that matches the models::RowSet type
    let mut json_select = sql::helpers::select_rowset(
        sql::helpers::make_column_alias("universe".to_string()),
        sql::helpers::make_table_alias("universe".to_string()),
        sql::helpers::make_table_alias("rows".to_string()),
        sql::helpers::make_column_alias("rows".to_string()),
        sql::helpers::make_table_alias("aggregates".to_string()),
        sql::helpers::make_column_alias("aggregates".to_string()),
        select_set,
    );

    // add native queries if there are any
    json_select.with = sql::ast::With {
        common_table_expressions: native_queries::translate(state),
    };

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
    state: &mut State,
    current_table: &TableNameAndReference,
    from_clause: &sql::ast::From,
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
    let row_select: Option<sql::ast::Select> =
        root::translate_rows_query(env, state, current_table, from_clause, &query)
            .map_or_else(map_no_fields_error_to_none, wrap_ok)?;

    // translate aggregate select. if there are no fields, make this a None
    let aggregate_select: Option<sql::ast::Select> =
        root::translate_aggregate_query(env, state, current_table, from_clause, &query)
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
