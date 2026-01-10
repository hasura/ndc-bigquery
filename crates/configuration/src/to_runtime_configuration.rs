//! Convert the parsed configuration metadata to internal engine metadata
//! That can be used by the connector at runtime.

use std::collections::BTreeMap;

use super::version1::ParsedConfiguration;
use crate::environment::Environment;
use crate::error::MakeRuntimeConfigurationError;
use crate::values::{DatasetId, ProjectId, Secret, ServiceKey, WorkloadIdentityAuth};
use query_engine_metadata::{self, metadata};

/// Convert the parsed configuration metadata to internal engine metadata
/// That can be used by the connector at runtime.
pub fn make_runtime_configuration(
    parsed_config: ParsedConfiguration,
    environment: impl Environment,
) -> Result<crate::Configuration, MakeRuntimeConfigurationError> {
    let auth: (bool, String) = match parsed_config.connection_settings.workload_identity_auth {
        Some(WorkloadIdentityAuth(Secret::Plain(key))) => Ok((true, key)),
        Some(WorkloadIdentityAuth(Secret::FromEnvironment { variable })) => {
            let variable_value = environment.read(&variable).map_err(|error| {
                MakeRuntimeConfigurationError::MissingEnvironmentVariable {
                    file_path: super::version1::CONFIGURATION_FILENAME.into(),
                    message: error.to_string(),
                }
            })?;
            Ok((true, variable_value))
        }
        None => match parsed_config.connection_settings.service_key {
            Some(ServiceKey(Secret::Plain(key))) => Ok((false, key)),
            Some(ServiceKey(Secret::FromEnvironment { variable })) => {
                let variable_value = environment.read(&variable).map_err(|error| {
                    MakeRuntimeConfigurationError::MissingEnvironmentVariable {
                        file_path: super::version1::CONFIGURATION_FILENAME.into(),
                        message: error.to_string(),
                    }
                })?;
                Ok((false, variable_value))
            }
            None => {
                return Err(MakeRuntimeConfigurationError::MissingEnvironmentVariable {
                    file_path: super::version1::CONFIGURATION_FILENAME.into(),
                    message: "Neither Workload Identity Auth URL or Service key is provided".into(),
                });
            }
        },
    }?;

    let project_id = match parsed_config.connection_settings.project_id {
        ProjectId(Secret::Plain(key)) => Ok(key),
        ProjectId(Secret::FromEnvironment { variable }) => {
            environment.read(&variable).map_err(|error| {
                MakeRuntimeConfigurationError::MissingEnvironmentVariable {
                    file_path: super::version1::CONFIGURATION_FILENAME.into(),
                    message: error.to_string(),
                }
            })
        }
    }?;
    let dataset_id = match parsed_config.connection_settings.dataset_id {
        DatasetId(Secret::Plain(key)) => Ok(key),
        DatasetId(Secret::FromEnvironment { variable }) => {
            environment.read(&variable).map_err(|error| {
                MakeRuntimeConfigurationError::MissingEnvironmentVariable {
                    file_path: super::version1::CONFIGURATION_FILENAME.into(),
                    message: error.to_string(),
                }
            })
        }
    }?;
    Ok(crate::Configuration {
        metadata: convert_metadata(parsed_config.metadata),
        pool_settings: parsed_config.pool_settings,
        auth,
        project_id,
        dataset_id,
    })
}

/// Convert the metadata specified in the parsed configuration to an engine metadata.
/// This function is used by tests as well
pub fn convert_metadata(metadata: metadata::Metadata) -> query_engine_metadata::metadata::Metadata {
    query_engine_metadata::metadata::Metadata {
        tables: convert_tables(metadata.tables),
        scalar_types: convert_scalar_types(metadata.scalar_types),
        // composite_types: convert_composite_types(metadata.composite_types),
        native_operations: convert_native_operations(metadata.native_operations),
    }
}

fn convert_scalar_types(
    scalar_types: metadata::ScalarTypes,
) -> query_engine_metadata::metadata::ScalarTypes {
    query_engine_metadata::metadata::ScalarTypes(
        scalar_types
            .0
            .into_iter()
            .map(|(scalar_type_name, scalar_type)| {
                (
                    scalar_type_name,
                    query_engine_metadata::metadata::ScalarType {
                        type_name: scalar_type.type_name,
                        schema_name: (scalar_type.schema_name),
                        description: scalar_type.description,
                        aggregate_functions: scalar_type.aggregate_functions,
                        comparison_operators: scalar_type.comparison_operators,
                        type_representation: scalar_type.type_representation,
                    },
                )
            })
            .collect(),
    )
}

fn convert_native_operations(
    native_operations: metadata::NativeOperations,
) -> query_engine_metadata::metadata::NativeOperations {
    let mut queries = BTreeMap::new();
    let mut mutations = BTreeMap::new();

    for (name, query) in native_operations.queries.0 {
        queries.insert(name, convert_native_query_info(query));
    }
    for (name, mutation) in native_operations.mutations.0 {
        mutations.insert(name, convert_native_query_info(mutation));
    }

    query_engine_metadata::metadata::NativeOperations {
        queries: query_engine_metadata::metadata::NativeQueries(queries),
        mutations: query_engine_metadata::metadata::NativeMutations(mutations),
    }
}

fn convert_native_query_info(
    native_query_info: metadata::NativeQueryInfo,
) -> query_engine_metadata::metadata::NativeQueryInfo {
    query_engine_metadata::metadata::NativeQueryInfo {
        sql: convert_native_query_sql_either(native_query_info.sql),
        columns: native_query_info
            .columns
            .into_iter()
            .map(|(k, v)| (k, convert_read_only_column_info(v)))
            .collect(),
        arguments: native_query_info
            .arguments
            .into_iter()
            .map(|(k, v)| (k, convert_read_only_column_info(v)))
            .collect(),
        description: native_query_info.description,
    }
}

fn convert_read_only_column_info(
    read_only_column_info: metadata::ReadOnlyColumnInfo,
) -> query_engine_metadata::metadata::ReadOnlyColumnInfo {
    query_engine_metadata::metadata::ReadOnlyColumnInfo {
        name: read_only_column_info.name,
        r#type: read_only_column_info.r#type,
        nullable: convert_nullable(&read_only_column_info.nullable),
        description: read_only_column_info.description,
    }
}

fn convert_nullable(nullable: &metadata::Nullable) -> query_engine_metadata::metadata::Nullable {
    match nullable {
        metadata::Nullable::Nullable => query_engine_metadata::metadata::Nullable::Nullable,
        metadata::Nullable::NonNullable => query_engine_metadata::metadata::Nullable::NonNullable,
    }
}

fn convert_native_query_sql_either(
    sql: metadata::NativeQuerySqlEither,
) -> query_engine_metadata::metadata::NativeQuerySqlEither {
    match sql {
        metadata::NativeQuerySqlEither::NativeQuerySql(internal_sql) => {
            query_engine_metadata::metadata::NativeQuerySqlEither::NativeQuerySql(
                convert_native_query_sql_internal(internal_sql),
            )
        }
        metadata::NativeQuerySqlEither::NativeQuerySqlExternal(external_sql) => {
            query_engine_metadata::metadata::NativeQuerySqlEither::NativeQuerySqlExternal(
                convert_native_query_sql_external(external_sql),
            )
        }
    }
}

fn convert_native_query_sql_internal(
    internal_sql: metadata::NativeQuerySql,
) -> query_engine_metadata::metadata::NativeQuerySql {
    match internal_sql {
        metadata::NativeQuerySql::FromFile { file, sql } => {
            query_engine_metadata::metadata::NativeQuerySql::FromFile {
                file,
                sql: convert_native_query_parts(sql),
            }
        }
        metadata::NativeQuerySql::Inline { sql } => {
            query_engine_metadata::metadata::NativeQuerySql::Inline {
                sql: convert_native_query_parts(sql),
            }
        }
    }
}

fn convert_native_query_sql_external(
    external_sql: metadata::NativeQuerySqlExternal,
) -> query_engine_metadata::metadata::NativeQuerySqlExternal {
    match external_sql {
        metadata::NativeQuerySqlExternal::File { file } => {
            query_engine_metadata::metadata::NativeQuerySqlExternal::File { file }
        }
        metadata::NativeQuerySqlExternal::Inline { inline } => {
            query_engine_metadata::metadata::NativeQuerySqlExternal::Inline {
                inline: convert_native_query_parts(inline),
            }
        }
        metadata::NativeQuerySqlExternal::InlineUntagged(parts) => {
            query_engine_metadata::metadata::NativeQuerySqlExternal::InlineUntagged(
                convert_native_query_parts(parts),
            )
        }
    }
}

fn convert_native_query_parts(
    inline: metadata::NativeQueryParts,
) -> query_engine_metadata::metadata::NativeQueryParts {
    query_engine_metadata::metadata::NativeQueryParts(
        inline
            .0
            .into_iter()
            .map(convert_native_query_part)
            .collect(),
    )
}

fn convert_native_query_part(
    native_query_part: metadata::NativeQueryPart,
) -> query_engine_metadata::metadata::NativeQueryPart {
    match native_query_part {
        metadata::NativeQueryPart::Text(t) => {
            query_engine_metadata::metadata::NativeQueryPart::Text(t)
        }
        metadata::NativeQueryPart::Parameter(p) => {
            query_engine_metadata::metadata::NativeQueryPart::Parameter(p)
        }
    }
}

pub fn convert_tables(tables: metadata::TablesInfo) -> query_engine_metadata::metadata::TablesInfo {
    query_engine_metadata::metadata::TablesInfo(
        tables
            .0
            .into_iter()
            .map(|(k, table_info)| (k, convert_table_info(table_info)))
            .collect(),
    )
}

fn convert_table_info(
    table_info: metadata::TableInfo,
) -> query_engine_metadata::metadata::TableInfo {
    query_engine_metadata::metadata::TableInfo {
        schema_name: table_info.schema_name,
        table_name: table_info.table_name,
        columns: table_info
            .columns
            .into_iter()
            .map(|(k, column_info)| (k, convert_column_info(column_info)))
            .collect(),
        uniqueness_constraints: (table_info.uniqueness_constraints),
        foreign_relations: convert_foreign_relations(table_info.foreign_relations),
        description: table_info.description,
    }
}

fn convert_foreign_relations(
    foreign_relations: metadata::ForeignRelations,
) -> query_engine_metadata::metadata::ForeignRelations {
    query_engine_metadata::metadata::ForeignRelations(
        foreign_relations
            .0
            .into_iter()
            .map(|(k, foreign_relation)| (k, convert_foreign_relation(foreign_relation)))
            .collect(),
    )
}

fn convert_foreign_relation(
    foreign_relation: metadata::ForeignRelation,
) -> query_engine_metadata::metadata::ForeignRelation {
    query_engine_metadata::metadata::ForeignRelation {
        foreign_schema: foreign_relation.foreign_schema,
        foreign_table: foreign_relation.foreign_table,
        column_mapping: foreign_relation.column_mapping,
    }
}

// fn convert_uniqueness_constraints(
//     uniqueness_constraints: metadata::UniquenessConstraints,
// ) -> query_engine_metadata::metadata::UniquenessConstraints {
//     query_engine_metadata::metadata::UniquenessConstraints(
//         uniqueness_constraints
//             .0
//             .into_iter()
//             .map(|(k, uniqueness_constraint)| {
//                 (k, convert_uniqueness_constraint(uniqueness_constraint))
//             })
//             .collect(),
//     )
// }

// fn convert_uniqueness_constraint(
//     uniqueness_constraint: metadata::UniquenessConstraint,
// ) -> query_engine_metadata::metadata::UniquenessConstraint {
//     query_engine_metadata::metadata::UniquenessConstraint(
//         uniqueness_constraint
//             .0
//             .into_iter()
//             .map(|()| (c.to_string(), c))
//             .collect(),
//     )
// }

fn convert_column_info(
    column_info: metadata::ColumnInfo,
) -> query_engine_metadata::metadata::ColumnInfo {
    query_engine_metadata::metadata::ColumnInfo {
        name: column_info.name,
        r#type: column_info.r#type,
        nullable: convert_nullable(&column_info.nullable),
        has_default: convert_has_default(&column_info.has_default),
        is_identity: convert_is_identity(&column_info.is_identity),
        is_generated: convert_is_generated(&column_info.is_generated),
        description: column_info.description,
    }
}

fn convert_is_generated(
    is_generated: &metadata::IsGenerated,
) -> query_engine_metadata::metadata::IsGenerated {
    match is_generated {
        metadata::IsGenerated::NotGenerated => {
            query_engine_metadata::metadata::IsGenerated::NotGenerated
        }
        metadata::IsGenerated::Stored => query_engine_metadata::metadata::IsGenerated::Stored,
    }
}

fn convert_is_identity(
    is_identity: &metadata::IsIdentity,
) -> query_engine_metadata::metadata::IsIdentity {
    match is_identity {
        metadata::IsIdentity::NotIdentity => {
            query_engine_metadata::metadata::IsIdentity::NotIdentity
        }
        metadata::IsIdentity::IdentityByDefault => {
            query_engine_metadata::metadata::IsIdentity::IdentityByDefault
        }
        metadata::IsIdentity::IdentityAlways => {
            query_engine_metadata::metadata::IsIdentity::IdentityAlways
        }
    }
}

fn convert_has_default(
    has_default: &metadata::HasDefault,
) -> query_engine_metadata::metadata::HasDefault {
    match has_default {
        metadata::HasDefault::NoDefault => query_engine_metadata::metadata::HasDefault::NoDefault,
        metadata::HasDefault::HasDefault => query_engine_metadata::metadata::HasDefault::HasDefault,
    }
}

// fn convert_mutations_version(
//     mutations_version_opt: Option<metadata::mutations::MutationsVersion>,
// ) -> Option<query_engine_metadata::metadata::mutations::MutationsVersion> {
//     mutations_version_opt.map(|mutations_version| match mutations_version {
//         metadata::mutations::MutationsVersion::V1 => {
//             query_engine_metadata::metadata::mutations::MutationsVersion::V1
//         }
//         metadata::mutations::MutationsVersion::V2 => {
//             query_engine_metadata::metadata::mutations::MutationsVersion::V2
//         }
//     })
// }
