//! Handle stuff related to relationships and joins.

use super::error::Error;

use super::helpers::{RootAndCurrentTables, TableNameAndReference};
use crate::metadata;
use crate::phases::translation::sql;

use ndc_client::models;

use std::collections::BTreeMap;

/// translate any joins we should include in the query into our SQL AST
pub fn translate_joins(
    relationships: &BTreeMap<String, models::Relationship>,
    tables_info: &metadata::TablesInfo,
    root_and_current_tables: &RootAndCurrentTables,
    // We got these by processing the fields selection.
    join_fields: Vec<(sql::ast::TableAlias, String, models::Query)>,
) -> Result<Vec<sql::ast::Join>, Error> {
    // traverse and build a join.
    join_fields
        .into_iter()
        .map(|(alias, relationship_name, query)| {
            let relationship = relationships
                .get(&relationship_name)
                .ok_or(Error::RelationshipNotFound(relationship_name.clone()))?;

            // process inner query and get the SELECTs for the 'rows' and 'aggregates' fields.
            let select_set = super::translate_query(
                tables_info,
                relationship.target_collection.clone(),
                relationships,
                query,
            )?;

            // add join expressions to row / aggregate selects
            let final_select_set = match select_set {
                // Only rows
                sql::helpers::SelectSet::Rows(mut row_select) => {
                    let sql::ast::Where(row_expr) = row_select.where_;

                    row_select.where_ = sql::ast::Where(translate_column_mapping(
                        tables_info,
                        &root_and_current_tables.current_table,
                        row_expr,
                        relationship,
                    )?);

                    Ok(sql::helpers::SelectSet::Rows(row_select))
                }
                // Only aggregates
                sql::helpers::SelectSet::Aggregates(mut aggregate_select) => {
                    let sql::ast::Where(aggregate_expr) = aggregate_select.where_;

                    aggregate_select.where_ = sql::ast::Where(translate_column_mapping(
                        tables_info,
                        &root_and_current_tables.current_table,
                        aggregate_expr,
                        relationship,
                    )?);

                    Ok(sql::helpers::SelectSet::Aggregates(aggregate_select))
                }
                // Both
                sql::helpers::SelectSet::RowsAndAggregates(
                    mut row_select,
                    mut aggregate_select,
                ) => {
                    let sql::ast::Where(row_expr) = row_select.where_;

                    row_select.where_ = sql::ast::Where(translate_column_mapping(
                        tables_info,
                        &root_and_current_tables.current_table,
                        row_expr,
                        relationship,
                    )?);

                    let sql::ast::Where(aggregate_expr) = aggregate_select.where_;

                    aggregate_select.where_ = sql::ast::Where(translate_column_mapping(
                        tables_info,
                        &root_and_current_tables.current_table,
                        aggregate_expr,
                        relationship,
                    )?);

                    // Build (what will be) a RowSet with both fields.
                    Ok(sql::helpers::SelectSet::RowsAndAggregates(
                        row_select,
                        aggregate_select,
                    ))
                }
            }?;

            // form a single JSON item shaped `{ rows: [], aggregates: {} }`
            // that matches the models::RowSet type
            let json_select = sql::helpers::select_rowset(
                sql::helpers::make_column_alias(alias.name.clone()),
                sql::helpers::make_table_alias(alias.name.clone()),
                sql::helpers::make_table_alias("rows".to_string()),
                sql::helpers::make_column_alias("rows".to_string()),
                sql::helpers::make_table_alias("aggregates".to_string()),
                sql::helpers::make_column_alias("aggregates".to_string()),
                final_select_set,
            );

            Ok(sql::ast::Join::LeftOuterJoinLateral(
                sql::ast::LeftOuterJoinLateral {
                    select: Box::new(json_select),
                    alias,
                },
            ))
        })
        .collect::<Result<Vec<sql::ast::Join>, Error>>()
}

/// Given a relationship, turn it into a Where clause for a Join.
pub fn translate_column_mapping(
    tables_info: &metadata::TablesInfo,
    current_table: &TableNameAndReference,
    expr: sql::ast::Expression,
    relationship: &models::Relationship,
) -> Result<sql::ast::Expression, Error> {
    let metadata::TablesInfo(tables_info_map) = tables_info;

    let table_info = tables_info_map
        .get(&current_table.name)
        .ok_or(Error::TableNotFound(current_table.name.to_string()))?;

    let target_collection_info = tables_info_map
        .get(&relationship.target_collection)
        .ok_or(Error::TableNotFound(relationship.target_collection.clone()))?;
    let target_collection_alias: sql::ast::TableAlias =
        sql::helpers::make_table_alias(relationship.target_collection.clone());
    let target_collection_alias_name: sql::ast::TableName =
        sql::ast::TableName::AliasedTable(target_collection_alias);

    relationship
        .column_mapping
        .iter()
        .map(|(source_col, target_col)| {
            let source_column_info =
                table_info
                    .columns
                    .get(source_col)
                    .ok_or(Error::ColumnNotFoundInTable(
                        source_col.clone(),
                        current_table.name.clone(),
                    ))?;
            let target_column_info = target_collection_info.columns.get(target_col).ok_or(
                Error::ColumnNotFoundInTable(
                    target_col.clone(),
                    relationship.target_collection.clone(),
                ),
            )?;
            Ok(sql::ast::Expression::BinaryOperator {
                left: Box::new(sql::ast::Expression::ColumnName(
                    sql::ast::ColumnName::TableColumn {
                        table: sql::ast::TableName::AliasedTable(current_table.reference.clone()),
                        name: source_column_info.name.clone(),
                    },
                )),
                operator: sql::ast::BinaryOperator::Equals,
                right: Box::new(sql::ast::Expression::ColumnName(
                    sql::ast::ColumnName::TableColumn {
                        table: target_collection_alias_name.clone(),
                        name: target_column_info.name.clone(),
                    },
                )),
            })
        })
        .try_fold(expr, |expr, op| {
            let op = op?;
            Ok(sql::ast::Expression::And {
                left: Box::new(expr),
                right: Box::new(op),
            })
        })
}
