//! Handle 'rows' and 'aggregates' translation.

use indexmap::IndexMap;

use ndc_models as models;

use super::relationships;
use crate::translation::error::Error;
use crate::translation::error::UnsupportedCapabilities;
use crate::translation::helpers::FieldsInfo;
use crate::translation::helpers::{Env, State, TableNameAndReference};
use query_engine_metadata::metadata::{BigQueryType, Type, TypeRange};
use query_engine_sql::sql;
use std::collections::BTreeMap;

/// Translate the field-selection of a query to SQL.
/// Because field selection may be nested this function is mutually recursive with
/// 'translate_nested_field'.
pub(crate) fn translate_fields(
    env: &Env,
    state: &mut State,
    fields: IndexMap<models::FieldName, models::Field>,
    current_table: &TableNameAndReference,
    from: sql::ast::From,
    join_relationship_fields: &mut Vec<relationships::JoinFieldInfo>,
) -> Result<sql::ast::Select, Error> {
    // find the table according to the metadata.
    let fields_info = env.lookup_fields_info(&current_table.name)?;

    let columns: Vec<(sql::ast::ColumnAlias, sql::ast::Expression)> = fields
        .into_iter()
        .map(|(alias, field)| match field {
            models::Field::Column {
                column,
                fields: None,
                arguments,
            } if arguments.is_empty() => unpack_and_wrap_fields(
                env,
                current_table,
                &column,
                sql::helpers::make_column_alias(alias.to_string()),
                &fields_info,
            ),
            models::Field::Column {
                column: _,
                fields: _,
                arguments: _,
            } => Err(Error::CapabilityNotSupported(
                UnsupportedCapabilities::FieldArguments,
            )),
            models::Field::Relationship {
                query,
                relationship,
                arguments,
            } => {
                let table_alias = state.make_relationship_table_alias(alias.as_str());
                let column_alias = sql::helpers::make_column_alias(alias.to_string());
                let column_name = sql::ast::ColumnReference::AliasedColumn {
                    table: sql::ast::TableReference::AliasedTable(table_alias.clone()),
                    column: column_alias.clone(),
                };
                join_relationship_fields.push(relationships::JoinFieldInfo {
                    table_alias,
                    column_alias: column_alias.clone(),
                    relationship_name: relationship,
                    arguments,
                    query: *query,
                });
                Ok((
                    column_alias,
                    sql::ast::Expression::ColumnReference(column_name),
                ))
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let mut select = sql::helpers::simple_select(columns);

    select.from = Some(from);

    // let select_final = match returns_field {
    //     ReturnsFields::FieldsWereRequested => {
    //         (select)
    //     }
    //     ReturnsFields::NoFieldsWereRequested => {
    //         // If fields were requested, we need to return the fields as they are.
    //         // This is the default behavior.
    //         let select_1 = sql::ast::SelectList::Select1;
    //         let select = sql::ast::Select {
    //             with: sql::helpers::empty_with(),
    //             select_list: select_1,
    //             from: Some(from),
    //             joins: vec![],
    //             where_: sql::ast::Where(sql::helpers::empty_where()),
    //             group_by:sql::helpers::empty_group_by(),
    //             order_by: sql::helpers::empty_order_by(),
    //             limit: sql::helpers::empty_limit(),
    //         };
    //         select
    //     }
    // };

    Ok(select)
}

#[allow(clippy::too_many_arguments)]
/// In order to return the expected type representation for each column,
/// we need to wrap columns in type representation cast, and unpack composite types
/// so we can wrap them.
fn unpack_and_wrap_fields(
    env: &Env,
    current_table: &TableNameAndReference,
    column: &models::FieldName,
    alias: sql::ast::ColumnAlias,
    fields_info: &FieldsInfo<'_>,
) -> Result<(sql::ast::ColumnAlias, sql::ast::Expression), Error> {
    let column_info = fields_info.lookup_column(column)?;

    // Different kinds of types have different strategy for converting to their
    // type representation.
    match column_info.r#type {
        // Scalar types can just be wrapped in a cast.
        Type::ScalarType(scalar_type) => uppack_and_wrap_fields_scalar_type(
            env,
            current_table,
            column,
            fields_info,
            &scalar_type,
            alias,
        ),
        Type::StructType(fields) => unpack_and_wrap_fields_struct_type(
            env,
            current_table,
            column,
            fields_info,
            fields,
            alias,
        ),
        Type::RangeType(range_type) => unpack_and_wrap_fields_range_type(
            current_table,
            column,
            fields_info,
            range_type.clone(),
            alias,
        ),
        Type::ArrayType(ref type_boxed) => match type_boxed.as_ref() {
            Type::ArrayType(_) => Err(Error::NestedArraysNotSupported {
                field_name: column.clone(),
            }),
            Type::StructType(fields) => unpack_and_wrap_fields_struct_type(
                env,
                current_table,
                column,
                fields_info,
                fields.clone(),
                alias,
            ),
            Type::RangeType(range_type) => unpack_and_wrap_fields_range_type(
                current_table,
                column,
                fields_info,
                range_type.clone(),
                alias,
            ),
            Type::ScalarType(ref scalar_type) => uppack_and_wrap_fields_scalar_type(
                env,
                current_table,
                column,
                fields_info,
                scalar_type,
                alias,
            ),
        },
    }
}

fn uppack_and_wrap_fields_scalar_type(
    env: &Env,
    current_table: &TableNameAndReference,
    column: &models::FieldName,
    fields_info: &FieldsInfo,
    scalar_type: &models::ScalarTypeName,
    alias: sql::ast::ColumnAlias,
) -> Result<(sql::ast::ColumnAlias, sql::ast::Expression), Error> {
    let column_info = fields_info.lookup_column(column)?;
    let column_type_representation = env.lookup_type_representation(&scalar_type);
    let (final_alias, expression) = sql::helpers::make_column(
        current_table.reference.clone(),
        column_info.name.clone(),
        alias,
    );
    Ok((
        final_alias,
        wrap_in_type_representation(expression, column_type_representation),
    ))
}

fn unpack_and_wrap_fields_struct_type(
    env: &Env,
    current_table: &TableNameAndReference,
    column: &models::FieldName,
    fields_info: &FieldsInfo,
    fields: BTreeMap<String, Type>,
    alias: sql::ast::ColumnAlias,
) -> Result<(sql::ast::ColumnAlias, sql::ast::Expression), Error> {
    let column_info = fields_info.lookup_column(column)?;
    let object_fields: Vec<(String, sql::ast::Expression)> = fields
        .into_iter()
        .map(|(field_name, field_type)| {
            let field_expr =
                sql::ast::Expression::ColumnReference(sql::ast::ColumnReference::TableColumn {
                    table: current_table.reference.clone(),
                    name: sql::ast::ColumnName(format!(
                        "{}.{}",
                        column_info.name.0.as_str(),
                        field_name
                    )),
                });
            let field_type_representation = match &field_type {
                Type::ScalarType(scalar_type) => env.lookup_type_representation(scalar_type),
                _ => None,
            };
            let field_expr = wrap_in_type_representation(field_expr, field_type_representation);

            (field_name, field_expr)
        })
        .collect();

    let json_object = sql::ast::Expression::FunctionCall {
        function: sql::ast::Function::Unknown("JSON_OBJECT".to_string()),
        args: object_fields
            .into_iter()
            .flat_map(|(k, v)| vec![sql::ast::Expression::Value(sql::ast::Value::String(k)), v])
            .collect(),
    };

    Ok((alias, json_object))
}

fn unpack_and_wrap_fields_range_type(
    current_table: &TableNameAndReference,
    column: &models::FieldName,
    fields_info: &FieldsInfo,
    range_type: TypeRange,
    alias: sql::ast::ColumnAlias,
) -> Result<(sql::ast::ColumnAlias, sql::ast::Expression), Error> {
    let column_info = fields_info.lookup_column(column)?;
    let column_ref =
        sql::ast::Expression::ColumnReference(sql::ast::ColumnReference::TableColumn {
            table: current_table.reference.clone(),
            name: column_info.name.clone(),
        });

    let start_expr = sql::ast::Expression::FunctionCall {
        function: sql::ast::Function::Unknown("RANGE_START".to_string()),
        args: vec![column_ref.clone()],
    };
    let end_expr = sql::ast::Expression::FunctionCall {
        function: sql::ast::Function::Unknown("RANGE_END".to_string()),
        args: vec![column_ref],
    };

    // Determine the appropriate type based on range_type
    let type_name = match range_type {
        TypeRange::Date => "DATE",
        TypeRange::Datetime => "DATETIME",
        TypeRange::Timestamp => "TIMESTAMP",
    };

    let json_object = sql::ast::Expression::FunctionCall {
        function: sql::ast::Function::Unknown("JSON_OBJECT".to_string()),
        args: vec![
            sql::ast::Expression::Value(sql::ast::Value::String("start".to_string())),
            sql::ast::Expression::Cast {
                expression: Box::new(start_expr),
                r#type: sql::ast::ScalarType::BaseType(sql::ast::ScalarTypeName::Unqualified(
                    type_name.to_string(),
                )),
            },
            sql::ast::Expression::Value(sql::ast::Value::String("end".to_string())),
            sql::ast::Expression::Cast {
                expression: Box::new(end_expr),
                r#type: sql::ast::ScalarType::BaseType(sql::ast::ScalarTypeName::Unqualified(
                    type_name.to_string(),
                )),
            },
        ],
    };

    Ok((alias, json_object))
}

/// Certain type representations require that we provide a different json representation
/// than what postgres will return.
/// For columns of those type representation, we wrap the result in a cast.
fn wrap_in_type_representation(
    expression: sql::ast::Expression,
    column_type_representation: Option<&BigQueryType>,
) -> sql::ast::Expression {
    match column_type_representation {
        None => expression,
        Some(type_rep) => {
            if let Some(cast_type) = get_type_representation_cast_type(type_rep) {
                sql::ast::Expression::Cast {
                    expression: Box::new(expression),
                    r#type: sql::ast::ScalarType::BaseType(cast_type),
                }
            } else {
                expression
            }
        }
    }
}

/// If a type representation requires a cast, return the scalar type name.
fn get_type_representation_cast_type(
    type_representation: &BigQueryType,
) -> Option<sql::ast::ScalarTypeName> {
    match type_representation {
        // In these situations, we expect to cast the expression according
        // to the type representation.
        BigQueryType::Bytes
        | BigQueryType::Int64
        | BigQueryType::Numeric
        | BigQueryType::BigNumeric => Some(sql::helpers::text_type_name()),

        // In these situations the type representation should be the same as
        // the expression, so we don't cast it.
        BigQueryType::Boolean
        | BigQueryType::String
        | BigQueryType::Float64
        | BigQueryType::Timestamp
        | BigQueryType::Time
        | BigQueryType::Date
        | BigQueryType::Datetime
        | BigQueryType::Geography
        | BigQueryType::Struct(_)
        | BigQueryType::Json
        | BigQueryType::Range(_)
        | BigQueryType::Array(_) => None,
    }
}
