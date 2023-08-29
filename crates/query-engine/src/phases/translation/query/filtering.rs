//! Handle filtering/where clauses translation.

use ndc_sdk::models;

use super::error::Error;
use super::helpers::{CollectionInfo, Env, RootAndCurrentTables, State, TableNameAndReference};
use super::relationships;
use super::values;
use crate::metadata;
use crate::phases::translation::sql;
use crate::phases::translation::sql::helpers::simple_select;

/// Translate a boolean expression to a SQL expression.
pub fn translate_expression(
    env: &Env,
    state: &mut State,
    next_free_name: &mut u32,
    root_and_current_tables: &RootAndCurrentTables,
    predicate: models::Expression,
) -> Result<(sql::ast::Expression, Vec<sql::ast::Join>), Error> {
    match predicate {
        models::Expression::And { expressions } => {
            let mut acc_joins = vec![];
            let and_exprs = expressions
                .into_iter()
                .map(|expr| {
                    translate_expression(env, state, next_free_name, root_and_current_tables, expr)
                })
                .try_fold(
                    sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
                    |acc, expr| {
                        let (right, right_joins) = expr?;
                        acc_joins.extend(right_joins);
                        Ok(sql::ast::Expression::And {
                            left: Box::new(acc),
                            right: Box::new(right),
                        })
                    },
                )?;
            Ok((and_exprs, acc_joins))
        }
        models::Expression::Or { expressions } => {
            let mut acc_joins = vec![];
            let or_exprs = expressions
                .into_iter()
                .map(|expr| {
                    translate_expression(env, state, next_free_name, root_and_current_tables, expr)
                })
                .try_fold(
                    sql::ast::Expression::Value(sql::ast::Value::Bool(true)),
                    |acc, expr| {
                        let (right, right_joins) = expr?;
                        acc_joins.extend(right_joins);
                        Ok(sql::ast::Expression::Or {
                            left: Box::new(acc),
                            right: Box::new(right),
                        })
                    },
                )?;
            Ok((or_exprs, acc_joins))
        }
        models::Expression::Not { expression } => {
            let (expr, joins) = translate_expression(
                env,
                state,
                next_free_name,
                root_and_current_tables,
                *expression,
            )?;
            Ok((sql::ast::Expression::Not(Box::new(expr)), joins))
        }
        models::Expression::BinaryComparisonOperator {
            column,
            operator,
            value,
        } => {
            let mut joins = vec![];
            let (left, left_joins) = translate_comparison_target(
                env,
                state,
                next_free_name,
                root_and_current_tables,
                *column,
            )?;
            let (right, right_joins) = translate_comparison_value(
                env,
                state,
                next_free_name,
                root_and_current_tables,
                *value,
            )?;

            joins.extend(left_joins);
            joins.extend(right_joins);
            Ok((
                sql::ast::Expression::BinaryOperator {
                    left: Box::new(left),
                    operator: match *operator {
                        models::BinaryComparisonOperator::Equal => sql::ast::BinaryOperator::Equals,
                        models::BinaryComparisonOperator::Other { name } =>
                        // The strings we're matching against here (ie 'like')
                        // are best guesses for now. We will need to update
                        // these as we discover more.
                        // We need to keep these in sync with documentation.
                        {
                            match name.as_str() {
                                "eq" => sql::ast::BinaryOperator::Equals,
                                "neq" => sql::ast::BinaryOperator::NotEquals,
                                "lt" => sql::ast::BinaryOperator::LessThan,
                                "lte" => sql::ast::BinaryOperator::LessThanOrEqualTo,
                                "gt" => sql::ast::BinaryOperator::GreaterThan,
                                "gte" => sql::ast::BinaryOperator::GreaterThanOrEqualTo,
                                "like" => sql::ast::BinaryOperator::Like,
                                "nlike" => sql::ast::BinaryOperator::NotLike,
                                "ilike" => sql::ast::BinaryOperator::CaseInsensitiveLike,
                                "nilike" => sql::ast::BinaryOperator::NotCaseInsensitiveLike,
                                "similar" => sql::ast::BinaryOperator::Similar,
                                "nsimilar" => sql::ast::BinaryOperator::NotSimilar,
                                "regex" => sql::ast::BinaryOperator::Regex,
                                "nregex" => sql::ast::BinaryOperator::NotRegex,
                                "iregex" => sql::ast::BinaryOperator::CaseInsensitiveRegex,
                                "niregex" => sql::ast::BinaryOperator::NotCaseInsensitiveRegex,
                                _ => sql::ast::BinaryOperator::Equals,
                            }
                        }
                    },
                    right: Box::new(right),
                },
                joins,
            ))
        }
        models::Expression::BinaryArrayComparisonOperator {
            column,
            operator,
            values,
        } => {
            let mut joins = vec![];
            let (left, left_joins) = translate_comparison_target(
                env,
                state,
                next_free_name,
                root_and_current_tables,
                *column.clone(),
            )?;
            joins.extend(left_joins);
            let right = values
                .iter()
                .map(|value| {
                    let (right, right_joins) = translate_comparison_value(
                        env,
                        state,
                        next_free_name,
                        root_and_current_tables,
                        value.clone(),
                    )?;
                    joins.extend(right_joins);
                    Ok(right)
                })
                .collect::<Result<Vec<sql::ast::Expression>, Error>>()?;

            Ok((
                sql::ast::Expression::BinaryArrayOperator {
                    left: Box::new(left),
                    operator: match *operator {
                        models::BinaryArrayComparisonOperator::In => {
                            sql::ast::BinaryArrayOperator::In
                        }
                    },
                    right,
                },
                joins,
            ))
        }

        models::Expression::Exists {
            in_collection,
            predicate,
        } => Ok((
            translate_exists_in_collection(
                env,
                state,
                next_free_name,
                root_and_current_tables,
                *in_collection,
                *predicate,
            )?,
            vec![],
        )),
        models::Expression::UnaryComparisonOperator { column, operator } => match *operator {
            models::UnaryComparisonOperator::IsNull => {
                let (value, joins) = translate_comparison_target(
                    env,
                    state,
                    next_free_name,
                    root_and_current_tables,
                    *column,
                )?;

                Ok((
                    sql::ast::Expression::UnaryOperator {
                        column: Box::new(value),
                        operator: sql::ast::UnaryOperator::IsNull,
                    },
                    joins,
                ))
            }
        },
    }
}

/// Given a vector of PathElements and the table alias for the table the
/// expression is over, we return a join in the form of:
///
///   INNER JOIN LATERAL
///   (
///     SELECT *
///     FROM
///       <table of path[0]> AS <fresh name>
///     WHERE
///       <table 0 join condition>
///       AND <predicate of path[0]>
///     AS <fresh name>
///   )
///   INNER JOIN LATERAL
///   (
///     SELECT *
///     FROM
///        <table of path[1]> AS <fresh name>
///     WHERE
///        <table 1 join condition on table 0>
///        AND <predicate of path[1]>
///   ) AS <fresh name>
///   ...
///   INNER JOIN LATERAL
///   (
///       SELECT *
///       FROM
///          <table of path[m]> AS <fresh name>
///       WHERE
///          <table m join condition on table m-1>
///          AND <predicate of path[m]>
///   ) AS <fresh name>
///
/// and the aliased table name under which the sought colum can be found, i.e.
/// the last drawn fresh name. Or, in the case of an empty paths vector, simply
/// the alias that was input.
///
fn translate_comparison_pathelements(
    env: &Env,
    state: &mut State,
    next_free_name: &mut u32,
    root_and_current_tables: &RootAndCurrentTables,
    path: Vec<models::PathElement>,
) -> Result<(TableNameAndReference, Vec<sql::ast::Join>), Error> {
    let mut joins = vec![];
    let RootAndCurrentTables { current_table, .. } = root_and_current_tables;

    let final_ref = path.into_iter().try_fold(
        current_table.clone(),
        |current_table_ref,
         models::PathElement {
             relationship,
             predicate,
             ..
         }| {
            // get the relationship table
            let relationship = env.lookup_relationship(&relationship)?;

            // I don't expect v3-engine to let us down, but just in case :)
            if current_table_ref.name != relationship.source_collection_or_type {
                Err(Error::CollectionNotFound(
                    relationship.source_collection_or_type.clone(),
                ))
            } else {
                Ok(())
            }?;

            let collection_info = env.lookup_collection(&relationship.target_collection)?;

            // unpack table info for now.
            let table_info = match collection_info {
                CollectionInfo::Table { info, .. } => Ok(info),
                CollectionInfo::NativeQuery { .. } => {
                    Err(Error::NotSupported("Native Queries".to_string()))
                }
            }?;

            // relationship target db table name
            let db_table_name: sql::ast::TableName = sql::ast::TableName::DBTable {
                schema: table_info.schema_name.clone(),
                table: table_info.table_name.clone(),
            };

            // new alias for the target table
            let target_table_alias: sql::ast::TableAlias =
                sql::helpers::make_boolean_expression_table_alias(
                    next_free_name,
                    &relationship.target_collection.clone().to_string(),
                );
            let target_table_alias_name =
                sql::ast::TableName::AliasedTable(target_table_alias.clone());

            // build a SELECT querying this table with the relevant predicate.
            let mut select = simple_select(vec![]);
            select.from = Some(sql::ast::From::Table {
                name: db_table_name.clone(),
                alias: target_table_alias.clone(),
            });

            select.select_list = sql::ast::SelectList::SelectStar;

            let new_root_and_current_tables = RootAndCurrentTables {
                root_table: root_and_current_tables.root_table.clone(),
                current_table: TableNameAndReference {
                    reference: target_table_alias_name.clone(),
                    name: relationship.target_collection.clone(),
                },
            };
            // relationship-specfic filter
            let (rel_cond, rel_joins) = translate_expression(
                env,
                state,
                next_free_name,
                &new_root_and_current_tables,
                *predicate,
            )?;

            // relationship where clause
            let cond = relationships::translate_column_mapping(
                env,
                &current_table_ref,
                &target_table_alias_name,
                rel_cond,
                relationship,
            )?;

            select.where_ = sql::ast::Where(cond);

            select.joins = rel_joins;

            joins.push(sql::ast::Join::InnerJoinLateral(
                sql::ast::InnerJoinLateral {
                    select: Box::new(select),
                    alias: target_table_alias.clone(),
                },
            ));
            Ok(new_root_and_current_tables.current_table)
        },
    )?;

    Ok((final_ref, joins))
}

/// translate a comparison target.
fn translate_comparison_target(
    env: &Env,
    state: &mut State,
    next_free_name: &mut u32,
    root_and_current_tables: &RootAndCurrentTables,
    column: models::ComparisonTarget,
) -> Result<(sql::ast::Expression, Vec<sql::ast::Join>), Error> {
    match column {
        models::ComparisonTarget::Column { name, path } => {
            let (table_ref, joins) = translate_comparison_pathelements(
                env,
                state,
                next_free_name,
                root_and_current_tables,
                path,
            )?;

            // get the unrelated table information from the metadata.
            let collection_info = env.lookup_collection(&table_ref.name)?;
            let metadata::ColumnInfo { name, .. } = collection_info.lookup_column(&name)?;

            Ok((
                sql::ast::Expression::ColumnName(sql::ast::ColumnName::TableColumn {
                    table: table_ref.reference.clone(),
                    name: name.to_string(),
                }),
                joins,
            ))
        }

        // Compare a column from the root table.
        models::ComparisonTarget::RootCollectionColumn { name } => {
            let RootAndCurrentTables { root_table, .. } = root_and_current_tables;
            // get the unrelated table information from the metadata.
            let collection_info = env.lookup_collection(&root_table.name)?;

            // find the requested column in the tables columns.
            let metadata::ColumnInfo { name, .. } = collection_info.lookup_column(&name)?;

            Ok((
                sql::ast::Expression::ColumnName(sql::ast::ColumnName::TableColumn {
                    table: root_table.reference.clone(),
                    name: name.to_string(),
                }),
                vec![],
            ))
        }
    }
}

/// translate a comparison value.
fn translate_comparison_value(
    env: &Env,
    state: &mut State,
    next_free_name: &mut u32,
    root_and_current_tables: &RootAndCurrentTables,
    value: models::ComparisonValue,
) -> Result<(sql::ast::Expression, Vec<sql::ast::Join>), Error> {
    match value {
        models::ComparisonValue::Column { column } => translate_comparison_target(
            env,
            state,
            next_free_name,
            root_and_current_tables,
            *column,
        ),
        models::ComparisonValue::Scalar { value: json_value } => Ok((
            sql::ast::Expression::Value(values::translate_json_value(&json_value)?),
            vec![],
        )),
        models::ComparisonValue::Variable { name: var } => Ok((
            sql::ast::Expression::Value(sql::ast::Value::Variable(var)),
            vec![],
        )),
    }
}

/// Translate an EXISTS clause into a SQL subquery of the following form:
///
/// > EXISTS (SELECT FROM <table> AS <alias> WHERE <predicate>)
pub fn translate_exists_in_collection(
    env: &Env,
    state: &mut State,
    next_free_name: &mut u32,
    root_and_current_tables: &RootAndCurrentTables,
    in_collection: models::ExistsInCollection,
    predicate: models::Expression,
) -> Result<sql::ast::Expression, Error> {
    match in_collection {
        // ignore arguments for now
        models::ExistsInCollection::Unrelated { collection, .. } => {
            // get the unrelated table information from the metadata.
            let collection_info = env.lookup_collection(&collection)?;

            // unpack table info for now.
            let table_info = match collection_info {
                CollectionInfo::Table { info, .. } => Ok(info),
                CollectionInfo::NativeQuery { .. } => {
                    Err(Error::NotSupported("Native Queries".to_string()))
                }
            }?;

            // db table name
            let db_table_name: sql::ast::TableName = sql::ast::TableName::DBTable {
                schema: table_info.schema_name.clone(),
                table: table_info.table_name.clone(),
            };

            // new alias for the table
            let table_alias: sql::ast::TableAlias =
                sql::helpers::make_table_alias(collection.clone());
            let table_alias_name: sql::ast::TableName =
                sql::ast::TableName::AliasedTable(table_alias.clone());

            // build a SELECT querying this table with the relevant predicate.
            let mut select = simple_select(vec![]);
            select.from = Some(sql::ast::From::Table {
                name: db_table_name.clone(),
                alias: table_alias.clone(),
            });

            let new_root_and_current_tables = RootAndCurrentTables {
                root_table: root_and_current_tables.root_table.clone(),
                current_table: TableNameAndReference {
                    reference: table_alias_name,
                    name: collection.clone(),
                },
            };

            let (expr, expr_joins) = translate_expression(
                env,
                state,
                next_free_name,
                &new_root_and_current_tables,
                predicate,
            )?;
            select.where_ = sql::ast::Where(expr);

            select.joins = expr_joins;

            // > EXISTS (SELECT FROM <table> AS <alias> WHERE <predicate>)
            Ok(sql::ast::Expression::Exists {
                select: Box::new(select),
            })
        }
        // We get a relationship name in exists, query the target table directly,
        // and build a WHERE clause that contains the join conditions and the specified
        // EXISTS condition.
        models::ExistsInCollection::Related { relationship, .. } => {
            // get the relationship table
            let relationship = env.lookup_relationship(&relationship)?;

            // I don't expect v3-engine to let us down, but just in case :)
            if root_and_current_tables.current_table.name != relationship.source_collection_or_type
            {
                Err(Error::CollectionNotFound(
                    relationship.source_collection_or_type.clone(),
                ))
            } else {
                Ok(())
            }?;

            // get the unrelated table information from the metadata.
            let collection_info = env.lookup_collection(&relationship.target_collection)?;
            // unpack table info for now.
            let table_info = match collection_info {
                CollectionInfo::Table { info, .. } => Ok(info),
                CollectionInfo::NativeQuery { .. } => {
                    Err(Error::NotSupported("Native Queries".to_string()))
                }
            }?;

            // relationship target db table name
            let db_table_name: sql::ast::TableName = sql::ast::TableName::DBTable {
                schema: table_info.schema_name.clone(),
                table: table_info.table_name.clone(),
            };

            // new alias for the target table
            let table_alias: sql::ast::TableAlias =
                sql::helpers::make_table_alias(relationship.target_collection.clone());
            let table_alias_name: sql::ast::TableName =
                sql::ast::TableName::AliasedTable(table_alias.clone());

            // build a SELECT querying this table with the relevant predicate.
            let mut select = simple_select(vec![]);
            select.from = Some(sql::ast::From::Table {
                name: db_table_name.clone(),
                alias: table_alias.clone(),
            });

            let new_root_and_current_tables = RootAndCurrentTables {
                root_table: root_and_current_tables.root_table.clone(),
                current_table: TableNameAndReference {
                    reference: table_alias_name.clone(),
                    name: relationship.target_collection.clone(),
                },
            };

            // exists condition
            let (exists_cond, exists_joins) = translate_expression(
                env,
                state,
                next_free_name,
                &new_root_and_current_tables,
                predicate,
            )?;

            // relationship where clause
            let cond = relationships::translate_column_mapping(
                env,
                &root_and_current_tables.current_table,
                &table_alias_name,
                exists_cond,
                relationship,
            )?;

            select.where_ = sql::ast::Where(cond);

            select.joins = exists_joins;

            // > EXISTS (SELECT FROM <table> AS <alias> WHERE <predicate>)
            Ok(sql::ast::Expression::Exists {
                select: Box::new(select),
            })
        }
    }
}
