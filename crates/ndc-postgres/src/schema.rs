use std::collections::BTreeMap;

use ndc_hub::connector;
use ndc_hub::models;

use super::configuration;

/// Get the connector's schema.
///
/// This function implements the [schema endpoint](https://hasura.github.io/ndc-spec/specification/schema/index.html)
/// from the NDC specification.
pub async fn get_schema(
    configuration::DeploymentConfiguration {
        metadata,
        aggregate_functions,
        ..
    }: &configuration::DeploymentConfiguration,
) -> Result<models::SchemaResponse, connector::SchemaError> {
    // TODO: Technically, we should list all scalar types regardless of
    // whether or not they have associated aggregate functions. However, as
    // we only deal with two for now, and we can guarantee that they always
    // have aggregate functions, we don't need to worry about this quite
    // yet.
    let mut scalar_types: BTreeMap<String, models::ScalarType> = aggregate_functions
        .0
        .iter()
        .map(|(scalar_type, aggregate_functions)| {
            (
                scalar_type.to_string(),
                models::ScalarType {
                    aggregate_functions: aggregate_functions
                        .iter()
                        .map(|(function_name, function_definition)| {
                            (
                                function_name.clone(),
                                models::AggregateFunctionDefinition {
                                    result_type: models::Type::Named {
                                        name: function_definition.return_type.to_string(),
                                    },
                                },
                            )
                        })
                        .collect(),
                    comparison_operators: BTreeMap::new(),
                    update_operators: BTreeMap::new(),
                },
            )
        })
        .collect();

    // Used for types we don't yet support
    scalar_types.insert(
        "any".into(),
        models::ScalarType {
            aggregate_functions: BTreeMap::new(),
            comparison_operators: BTreeMap::new(),
            update_operators: BTreeMap::new(),
        },
    );

    let collections = metadata
        .tables
        .0
        .iter()
        .map(|(table_name, table)| models::CollectionInfo {
            name: table_name.clone(),
            description: None,
            arguments: BTreeMap::new(),
            collection_type: table_name.clone(),
            insertable_columns: None,
            updatable_columns: None,
            deletable: false,
            uniqueness_constraints: table
                .uniqueness_constraints
                .0
                .iter()
                .map(
                    |(
                        constraint_name,
                        query_engine::metadata::UniquenessConstraint(constraint_columns),
                    )| {
                        (
                            constraint_name.clone(),
                            models::UniquenessConstraint {
                                unique_columns: constraint_columns.iter().cloned().collect(),
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
                        query_engine::metadata::ForeignRelation {
                            foreign_table,
                            column_mapping,
                        },
                    )| {
                        (
                            constraint_name.clone(),
                            models::ForeignKeyConstraint {
                                foreign_collection: foreign_table.clone(),
                                column_mapping: column_mapping.clone(),
                            },
                        )
                    },
                )
                .collect(),
        })
        .collect();

    let object_types = BTreeMap::from_iter(metadata.tables.0.iter().map(|(table_name, table)| {
        let object_type = models::ObjectType {
            description: None,
            fields: BTreeMap::from_iter(table.columns.values().map(|column| {
                (
                    column.name.clone(),
                    models::ObjectField {
                        arguments: BTreeMap::new(),
                        description: None,
                        r#type: models::Type::Named {
                            name: column.r#type.to_string(),
                        },
                    },
                )
            })),
        };
        (table_name.clone(), object_type)
    }));

    Ok(models::SchemaResponse {
        collections,
        procedures: vec![],
        functions: vec![],
        object_types,
        scalar_types,
    })
}