//! Handle stuff related to relationships and joins.

use super::error::Error;
use super::helpers;
use super::root;

use crate::metadata;
use crate::phases::translation::sql;

use ndc_client::models;

use std::collections::BTreeMap;

/// translate any joins we should include in the query into our SQL AST
pub fn translate_joins(
    relationships: &BTreeMap<String, models::Relationship>,
    tables_info: &metadata::TablesInfo,
    table_alias: &sql::ast::TableAlias,
    table_name: &str,
    join_fields: Vec<(sql::ast::TableAlias, String, models::Query)>,
) -> Result<Vec<sql::ast::Join>, Error> {
    join_fields
        .into_iter()
        .map(|(alias, relationship_name, query)| {
            let relationship = relationships
                .get(&relationship_name)
                .ok_or(Error::RelationshipNotFound(relationship_name.clone()))?;

            let mut select = root::translate_rows_query(
                tables_info,
                &relationship.target_collection,
                relationships,
                &query,
            )?;

            // apply join conditions
            let sql::ast::Where(expr) = select.where_;

            let with_join_condition =
                translate_column_mapping(tables_info, table_name, table_alias, expr, relationship)?;

            select.where_ = sql::ast::Where(with_join_condition);

            // when we want to get nested aggregates working, we should be using
            // `select_rowset` here instead so that we also generate selects for any aggregate
            // rows
            // we'll need to work out a way of generating unique table aliases that don't
            // collide with the top level ones first though
            let wrap_select = match relationship.relationship_type {
                // for some reason v3-engine expects object relationships
                // also in the form of a json array wrapped in `rows`.
                models::RelationshipType::Object => {
                    sql::helpers::select_table_as_json_array_in_rows_object
                }
                models::RelationshipType::Array => {
                    sql::helpers::select_table_as_json_array_in_rows_object
                }
            };

            // wrap the sql in row_to_json and json_agg
            let final_select = wrap_select(
                select,
                helpers::make_column_alias(alias.name.clone()),
                helpers::make_table_alias(alias.name.clone()),
            );

            Ok(sql::ast::Join::LeftOuterJoinLateral(
                sql::ast::LeftOuterJoinLateral {
                    select: Box::new(final_select),
                    alias,
                },
            ))
        })
        .collect::<Result<Vec<sql::ast::Join>, Error>>()
}

/// Given a relationship, turn it into a Where clause for a Join.
pub fn translate_column_mapping(
    tables_info: &metadata::TablesInfo,
    table_name: &str,
    table_alias: &sql::ast::TableAlias,
    expr: sql::ast::Expression,
    relationship: &models::Relationship,
) -> Result<sql::ast::Expression, Error> {
    let metadata::TablesInfo(tables_info_map) = tables_info;

    let table_info = tables_info_map
        .get(table_name)
        .ok_or(Error::TableNotFound(table_name.to_string()))?;

    let target_collection_info = tables_info_map
        .get(&relationship.target_collection)
        .ok_or(Error::TableNotFound(relationship.target_collection.clone()))?;
    let target_collection_alias: sql::ast::TableAlias =
        helpers::make_table_alias(relationship.target_collection.clone());
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
                        table_name.to_string(),
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
                        table: sql::ast::TableName::AliasedTable(table_alias.clone()),
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
