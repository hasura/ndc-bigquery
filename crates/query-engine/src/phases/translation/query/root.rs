//! Handle 'rows' and 'aggregates' translation.

use super::aggregates;
use super::error::Error;
use super::filtering;
use super::helpers;
use super::relationships;
use super::sorting;
use crate::phases::translation::sql;

use crate::metadata;

use indexmap::IndexMap;
use ndc_client::models;

use std::collections::BTreeMap;

/// Translate aggregates query to sql ast.
pub fn translate_aggregate_query(
    tables_info: &metadata::TablesInfo,
    table_name: &String,
    relationships: &BTreeMap<String, models::Relationship>,
    query: &models::Query,
) -> Result<sql::ast::Select, Error> {
    let table_alias: sql::ast::TableAlias = helpers::make_table_alias(table_name.clone());

    // translate aggregates to select list
    let aggregate_fields = query.aggregates.clone().ok_or(Error::NoFields)?;

    // fail if no aggregates defined at all
    match IndexMap::is_empty(&aggregate_fields) {
        true => Err(Error::NoFields),
        false => Ok(()),
    }?;

    // create all aggregate columns
    let aggregate_columns = aggregates::translate(
        sql::ast::TableName::AliasedTable(table_alias.clone()),
        aggregate_fields,
    )?;

    // create the select clause and the joins, order by, where clauses.
    // We don't add the limit afterwards.
    translate_query_part(
        tables_info,
        table_name,
        relationships,
        query,
        aggregate_columns,
        vec![],
    )
}

/// Translate rows part of query to sql ast.
pub fn translate_rows_query(
    tables_info: &metadata::TablesInfo,
    table_name: &String,
    relationships: &BTreeMap<String, models::Relationship>,
    query: &models::Query,
) -> Result<sql::ast::Select, Error> {
    let metadata::TablesInfo(tables_info_map) = tables_info;
    // find the table according to the metadata.
    let table_info = tables_info_map
        .get(table_name)
        .ok_or(Error::TableNotFound(table_name.clone()))?;
    let table_alias: sql::ast::TableAlias = helpers::make_table_alias(table_name.clone());
    let table_alias_name: sql::ast::TableName =
        sql::ast::TableName::AliasedTable(table_alias.clone());

    // join aliases
    let mut join_fields: Vec<(sql::ast::TableAlias, String, models::Query)> = vec![];

    // translate fields to select list
    let fields = query.fields.clone().ok_or(Error::NoFields)?;

    // fail if no columns defined at all
    match IndexMap::is_empty(&fields) {
        true => Err(Error::NoFields),
        false => Ok(()),
    }?;

    // translate fields to columns or relationships.
    let columns: Vec<(sql::ast::ColumnAlias, sql::ast::Expression)> = fields
        .into_iter()
        .map(|(alias, field)| match field {
            models::Field::Column { column, .. } => {
                let column_info = table_info
                    .columns
                    .get(&column)
                    .ok_or(Error::ColumnNotFoundInTable(column, table_name.clone()))?;
                Ok(helpers::make_column(
                    table_alias_name.clone(),
                    column_info.name.clone(),
                    helpers::make_column_alias(alias),
                ))
            }
            models::Field::Relationship {
                query,
                relationship,
                ..
            } => {
                let table_alias = helpers::make_table_alias(alias.clone());
                let column_alias = helpers::make_column_alias(alias);
                let column_name = sql::ast::ColumnName::AliasedColumn {
                    table: sql::ast::TableName::AliasedTable(table_alias.clone()),
                    name: column_alias.clone(),
                };
                join_fields.push((table_alias, relationship, *query));
                Ok((column_alias, sql::ast::Expression::ColumnName(column_name)))
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;

    // create the select clause and the joins, order by, where clauses.
    // We'll add the limit afterwards.
    let mut select = translate_query_part(
        tables_info,
        table_name,
        relationships,
        query,
        columns,
        join_fields,
    )?;

    // Add the limit.
    select.limit = sql::ast::Limit {
        limit: query.limit,
        offset: query.offset,
    };
    Ok(select)
}

/// Translate the lion (or common) part of 'rows' or 'aggregates' part of a query.
/// Specifically, from, joins, order bys, and where clauses.
///
/// This expects to get the relevant information about tables, relationships, the root table,
/// and the query, as well as the columns and join fields after processing.
///
/// One thing that this doesn't do that you want to do for 'rows' and not 'aggregates' is
/// set the limit and offset so you want to do that after calling this function.
fn translate_query_part(
    tables_info: &metadata::TablesInfo,
    table_name: &String,
    relationships: &BTreeMap<String, models::Relationship>,
    query: &models::Query,
    columns: Vec<(sql::ast::ColumnAlias, sql::ast::Expression)>,
    join_fields: Vec<(sql::ast::TableAlias, String, models::Query)>,
) -> Result<sql::ast::Select, Error> {
    let metadata::TablesInfo(tables_info_map) = tables_info;
    // find the table according to the metadata.
    let table_info = tables_info_map
        .get(table_name)
        .ok_or(Error::TableNotFound(table_name.clone()))?;
    let table: sql::ast::TableName = sql::ast::TableName::DBTable {
        schema: table_info.schema_name.clone(),
        table: table_info.table_name.clone(),
    };
    let table_alias: sql::ast::TableAlias = helpers::make_table_alias(table_name.clone());
    let table_alias_name: sql::ast::TableName =
        sql::ast::TableName::AliasedTable(table_alias.clone());

    // construct a simple select with the table name, alias, and selected columns.
    let mut select = sql::helpers::simple_select(columns);

    select.from = Some(sql::ast::From::Table {
        name: table,
        alias: table_alias.clone(),
    });

    // collect any joins for relationships
    let mut relationship_joins = relationships::translate_joins(
        relationships,
        tables_info,
        &table_alias,
        table_name,
        join_fields,
    )?;

    // translate order_by
    let (order_by, order_by_joins) = sorting::translate_order_by(
        tables_info,
        relationships,
        &table_alias,
        table_name,
        &query.order_by,
    )?;

    relationship_joins.extend(order_by_joins);

    select.joins = relationship_joins;

    select.order_by = order_by;

    // translate where
    select.where_ = sql::ast::Where(match query.clone().predicate {
        None => sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
        Some(predicate) => filtering::translate_expression(&table_alias_name, predicate),
    });

    Ok(select)
}
