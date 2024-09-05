//! Handle stuff related to relationships and joins.

use std::collections::BTreeMap;

use ndc_models as models;

use super::root;
use crate::translation::error::Error;
use crate::translation::helpers::{Env, State, TableNameAndReference};
use query_engine_sql::sql;

pub struct JoinFieldInfo {
    pub table_alias: sql::ast::TableAlias,
    pub column_alias: sql::ast::ColumnAlias,
    pub relationship_name: models::RelationshipName,
    pub arguments: BTreeMap<models::ArgumentName, models::RelationshipArgument>,
    pub query: models::Query,
}

/// translate any joins we should include in the query into our SQL AST
pub fn translate_joins(
    env: &Env,
    state: &mut State,
    current_table: &TableNameAndReference,
    // We got these by processing the fields selection.
    join_fields: Vec<JoinFieldInfo>,
) -> Result<Vec<sql::ast::Join>, Error> {
    // traverse and build a join.
    join_fields
        .into_iter()
        .map(|join_field| {
            let relationship = env.lookup_relationship(&join_field.relationship_name)?;
            let arguments = make_relationship_arguments(MakeRelationshipArguments {
                caller_arguments: join_field.arguments,
                relationship_arguments: relationship.arguments.clone(),
            })?;

            // process inner query and get the SELECTs for the 'rows' and 'aggregates' fields.
            let select_set = root::translate_query(
                env,
                state,
                &root::MakeFrom::Collection {
                    name: relationship.target_collection.clone(),
                    arguments,
                },
                // We ask to inject the join predicate into the where clause.
                &Some(root::JoinPredicate {
                    join_with: current_table,
                    relationship,
                }),
                &join_field.query,
            )?;

            // form a single JSON item shaped `{ rows: [], aggregates: {} }`
            // that matches the models::RowSet type
            let json_select = sql::helpers::select_rowset(
                // sql::helpers::ResultsKind::ObjectResults,
                (
                    join_field.table_alias.clone(),
                    join_field.column_alias.clone(),
                ),
                (
                    state.make_table_alias("rows".to_string()),
                    state.make_table_alias("rows_inner".to_string()),
                ),
                (
                    state.make_table_alias("aggregates".to_string()),
                    state.make_table_alias("aggregates_inner".to_string()),
                ),
                select_set,
            );

            Ok(sql::ast::Join::LeftOuterJoin(
                sql::ast::LeftOuterJoin {
                    select: Box::new(json_select),
                    alias: join_field.table_alias,
                    on: sql::ast::Expression::Value(sql::ast::Value::Bool(true)) // todo(PY): fixme
                },
            ))
        })
        .collect::<Result<Vec<sql::ast::Join>, Error>>()
}

/// Given a relationship, turn it into a Where clause for a Join.
pub fn translate_column_mapping(
    env: &Env,
    current_table: &TableNameAndReference,
    target_collection_alias_reference: &sql::ast::TableReference,
    expr: sql::ast::Expression,
    relationship: &models::Relationship,
) -> Result<sql::ast::Expression, Error> {
    let table_info = env.lookup_collection(&current_table.name)?;

    let target_collection_info = env.lookup_collection(&relationship.target_collection)?;

    relationship
        .column_mapping
        .iter()
        .map(|(source_col, target_col)| {
            let source_column_info = table_info.lookup_column(source_col)?;
            let target_column_info = target_collection_info.lookup_column(target_col)?;
            Ok(sql::ast::Expression::BinaryOperation {
                left: Box::new(sql::ast::Expression::ColumnReference(
                    sql::ast::ColumnReference::TableColumn {
                        table: current_table.reference.clone(),
                        name: source_column_info.name,
                    },
                )),
                operator: sql::ast::BinaryOperator("=".to_string()),
                right: Box::new(sql::ast::Expression::ColumnReference(
                    sql::ast::ColumnReference::TableColumn {
                        table: target_collection_alias_reference.clone(),
                        name: target_column_info.name,
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

#[derive(Debug)]
/// Used in `make_relationship_arguments()` below.
pub struct MakeRelationshipArguments {
    pub relationship_arguments: BTreeMap<models::ArgumentName, models::RelationshipArgument>,
    pub caller_arguments: BTreeMap<models::ArgumentName, models::RelationshipArgument>,
}

/// Combine the caller arguments and the relationship arguments into a single map.
///
/// We don't support relationships column arguments yet, so for now we convert to a regular argument
/// and throw an error on the column case. Will be fixed in the future.
pub fn make_relationship_arguments(
    arguments: MakeRelationshipArguments,
) -> Result<BTreeMap<models::ArgumentName, models::Argument>, Error> {
    // these are arguments defined in the relationship definition.
    let relationship_arguments: BTreeMap<models::ArgumentName, models::Argument> = arguments
        .relationship_arguments
        .into_iter()
        .map(|(key, argument)| Ok((key, relationship_argument_to_argument(argument)?)))
        .collect::<Result<BTreeMap<models::ArgumentName, models::Argument>, Error>>()?;

    // these are arguments defined when calling the relationship.
    let caller_arguments: BTreeMap<models::ArgumentName, models::Argument> = arguments
        .caller_arguments
        .into_iter()
        .map(|(key, argument)| Ok((key, relationship_argument_to_argument(argument)?)))
        .collect::<Result<BTreeMap<models::ArgumentName, models::Argument>, Error>>()?;

    let mut arguments = relationship_arguments;

    // We do not allow caller arguments to override relationship defined arguments,
    // because those might be specified as permissions.
    // We don't expect the engine to return such queries, but add this as a precaution.
    for (key, value) in caller_arguments {
        match arguments.insert(key.clone(), value) {
            None => Ok(()),
            Some(_) => Err(Error::RelationshipArgumentWasOverriden(key)),
        }?;
    }

    Ok(arguments)
}

/// We don't support relationships column arguments yet, so for now we convert to a regular argument
/// and throw an error on the column case. Will be fixed in the future.
fn relationship_argument_to_argument(
    argument: models::RelationshipArgument,
) -> Result<models::Argument, Error> {
    match argument {
        models::RelationshipArgument::Literal { value } => Ok(models::Argument::Literal { value }),
        models::RelationshipArgument::Variable { name } => Ok(models::Argument::Variable { name }),
        models::RelationshipArgument::Column { .. } => Err(Error::NotImplementedYet(
            "relationship column arguments".to_string(),
        )),
    }
}
