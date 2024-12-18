//! Implement the `/schema` endpoint to return the connector's schema.
//! See the Hasura
//! [Native Data Connector Specification](https://hasura.github.io/ndc-spec/specification/schema/index.html)
//! for further details.

use std::collections::BTreeMap;

use ndc_sdk::connector;
use ndc_sdk::models;
use query_engine_metadata::metadata;
use query_engine_metadata::metadata::OperatorKind;

use ndc_bigquery_configuration::configuration;

/// Get the connector's schema.
///
/// This function implements the [schema endpoint](https://hasura.github.io/ndc-spec/specification/schema/index.html)
/// from the NDC specification.
pub async fn get_schema(
    configuration: &configuration::Configuration,
) -> Result<models::SchemaResponse, connector::ErrorResponse> {
    let metadata = &configuration.metadata;
    let scalar_types: BTreeMap<models::ScalarTypeName, models::ScalarType> = metadata
        .scalar_types
        .0
        .iter()
        .map(|(scalar_type_name, scalar_type_info)| {
            let result = models::ScalarType {
                representation: scalar_type_info
                    .type_representation
                    .as_ref()
                    .map(map_type_representation),
                aggregate_functions: scalar_type_info
                    .aggregate_functions
                    .iter()
                    .map(|(function_name, function_definition)| {
                        (
                            function_name.clone(),
                            models::AggregateFunctionDefinition {
                                result_type: models::Type::Nullable {
                                    underlying_type: Box::new(models::Type::Named {
                                        name: function_definition.return_type.clone(),
                                    }),
                                },
                            },
                        )
                    })
                    .collect(),
                comparison_operators: scalar_type_info
                    .comparison_operators
                    .iter()
                    .map(|(op_name, op_def)| {
                        (
                            op_name.clone(),
                            match op_def.operator_kind {
                                OperatorKind::Equal => models::ComparisonOperatorDefinition::Equal,
                                OperatorKind::In => models::ComparisonOperatorDefinition::In,
                                OperatorKind::Custom => {
                                    models::ComparisonOperatorDefinition::Custom {
                                        argument_type: models::Type::Named {
                                            name: op_def.argument_type.as_str().into(),
                                        },
                                    }
                                }
                            },
                        )
                    })
                    .collect(),
            };
            (scalar_type_name.clone(), result)
        })
        .collect();

    let collections_by_identifier: BTreeMap<(&str, &str), &str> = metadata
        .tables
        .0
        .iter()
        .map(|(collection_name, table)| {
            (
                (table.schema_name.as_ref(), table.table_name.as_ref()),
                collection_name.as_str(),
            )
        })
        .collect();

    let collections = metadata
        .tables
        .0
        .iter()
        .map(|(table_name, table)| models::CollectionInfo {
            name: table_name.clone(),
            description: table.description.clone(),
            arguments: BTreeMap::new(),
            collection_type: table_name.as_str().into(),
            uniqueness_constraints: table
                .uniqueness_constraints
                .0
                .iter()
                .map(
                    |(constraint_name, metadata::UniquenessConstraint(constraint_columns))| {
                        (
                            constraint_name.clone(),
                            models::UniquenessConstraint {
                                unique_columns: constraint_columns.iter().cloned().collect(), // todo(PY): do we need different types for query engine and v1? BteeSet and BtreeMap
                            },
                        )
                    },
                )
                .collect(),
            foreign_keys: table
                .foreign_relations
                .0
                .iter()
                .map(
                    |(
                        constraint_name,
                        metadata::ForeignRelation {
                            foreign_schema,
                            foreign_table,
                            column_mapping,
                        },
                    )| {
                        (
                            constraint_name.clone(),
                            models::ForeignKeyConstraint {
                                foreign_collection: (*collections_by_identifier
                                    .get(&(
                                        // the foreign schema used to be implied, so if it is not
                                        // provided, we need to default back to the originating
                                        // table's schema
                                        foreign_schema.as_ref().unwrap_or(&table.schema_name),
                                        foreign_table,
                                    ))
                                    .unwrap_or_else(|| {
                                        panic!(
                                            "Unknown foreign table: {foreign_schema:?}.{foreign_table:?}"
                                        )
                                    }))
                                .into(),
                                column_mapping: column_mapping.clone(),
                            },
                        )
                    },
                )
                .collect(),
        })
        .collect();

    let object_types = metadata //BTreeMap::from_iter(metadata.tables.0.iter().map(|(table_name, table)| {
        .tables
        .0
        .iter()
        .map(|(table_name, table)| {
            let object_type = models::ObjectType {
                description: table.description.clone(),
                fields: table
                    .columns
                    .iter()
                    .map(|(column_name, column_info)| {
                        (
                            column_name.clone(),
                            models::ObjectField {
                                description: column_info.description.clone(),
                                r#type: column_to_type(column_info),
                                arguments: BTreeMap::new(),
                            },
                        )
                    })
                    .collect(),
            };
            (table_name.as_str().into(), object_type)
        })
        .collect::<BTreeMap<_, _>>();

    Ok(models::SchemaResponse {
        collections,
        procedures: vec![],
        functions: vec![],
        object_types,
        scalar_types,
    })
}

/// Map our local type representation to ndc-spec type representation.
fn map_type_representation(
    type_representation: &metadata::TypeRepresentation,
) -> models::TypeRepresentation {
    match type_representation {
        metadata::TypeRepresentation::Boolean => models::TypeRepresentation::Boolean,
        metadata::TypeRepresentation::Bytes => models::TypeRepresentation::Bytes,
        metadata::TypeRepresentation::String => models::TypeRepresentation::String,
        metadata::TypeRepresentation::Int64 => models::TypeRepresentation::Int64,
        metadata::TypeRepresentation::Float64 => models::TypeRepresentation::Float64,
        metadata::TypeRepresentation::Numeric => models::TypeRepresentation::BigDecimal,
        metadata::TypeRepresentation::BigNumeric => models::TypeRepresentation::BigDecimal,
        metadata::TypeRepresentation::Timestamp => models::TypeRepresentation::TimestampTZ,
        metadata::TypeRepresentation::Time => models::TypeRepresentation::String,
        metadata::TypeRepresentation::Date => models::TypeRepresentation::Date,
        metadata::TypeRepresentation::Datetime => models::TypeRepresentation::TimestampTZ,
        metadata::TypeRepresentation::Array(_) => models::TypeRepresentation::JSON,
        metadata::TypeRepresentation::Geography => models::TypeRepresentation::Geography,
        metadata::TypeRepresentation::Struct(_) => models::TypeRepresentation::JSON,
        metadata::TypeRepresentation::Json => models::TypeRepresentation::JSON,
        metadata::TypeRepresentation::Enum(variants) => models::TypeRepresentation::Enum {
            one_of: variants.clone(),
        },
    }
}

// ! Helper functions for generating ndc-spec schema objects.

// use ndc_sdk::models;

// use query_engine_metadata::metadata;

/// Extract the models::Type representation of a column.
pub fn column_to_type(column: &metadata::ColumnInfo) -> models::Type {
    match &column.nullable {
        metadata::Nullable::NonNullable => type_to_type(&column.r#type),
        metadata::Nullable::Nullable => models::Type::Nullable {
            underlying_type: Box::new(type_to_type(&column.r#type)),
        },
    }
}

/// Extract the models::Type representation of a readonly column.
pub fn readonly_column_to_type(column: &metadata::ReadOnlyColumnInfo) -> models::Type {
    match &column.nullable {
        metadata::Nullable::NonNullable => type_to_type(&column.r#type),
        metadata::Nullable::Nullable => models::Type::Nullable {
            underlying_type: Box::new(type_to_type(&column.r#type)),
        },
    }
}

pub fn type_to_type(typ: &metadata::Type) -> models::Type {
    match typ {
        metadata::Type::ArrayType(typ) => models::Type::Array {
            element_type: Box::new(type_to_type(typ)),
        },
        metadata::Type::ScalarType(scalar_type) => models::Type::Named {
            name: scalar_type.as_str().into(),
        },
    }
}
