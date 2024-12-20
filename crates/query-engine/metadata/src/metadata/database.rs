//! Metadata information regarding the database and tracked information.

use models::ScalarTypeName;
use ndc_models as models;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::collections::{BTreeMap, BTreeSet};

/// The scalar types supported by the Engine.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScalarTypeTypeName(pub String);

/// The type of values that a column, field, or argument may take.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Type {
    ScalarType(models::ScalarTypeName),
    ArrayType(Box<Type>),
    RangeType(TypeRange),
    StructType(BTreeMap<String, Type>),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TypeRange {
    Date,
    Datetime,
    Timestamp,
}

/// Map of all known/occurring scalar types.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScalarTypes(pub BTreeMap<models::ScalarTypeName, ScalarType>);

impl ScalarTypes {
    pub fn empty() -> Self {
        ScalarTypes(BTreeMap::new())
    }
}

/// Information about a scalar type. A scalar type is completely characterized by its name and the
/// operations you can do on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScalarType {
    pub type_name: ScalarTypeName,
    pub schema_name: Option<String>, // TODO(PY): see what happens when option is removed
    pub description: Option<String>,
    pub aggregate_functions: BTreeMap<models::AggregateFunctionName, AggregateFunction>,
    pub comparison_operators: BTreeMap<models::ComparisonOperatorName, ComparisonOperator>,
    pub type_representation: Option<BigQueryType>,
}

/// The complete list of supported binary operators for scalar types.
/// Not all of these are supported for every type.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonOperators(
    pub  BTreeMap<
        models::ScalarTypeName,
        BTreeMap<models::ComparisonOperatorName, ComparisonOperator>,
    >,
);

impl ComparisonOperators {
    pub fn empty() -> Self {
        ComparisonOperators(BTreeMap::new())
    }
}

/// Information about a composite type field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FieldInfo {
    pub field_name: String,
    pub r#type: Type,
    pub description: Option<String>,
}

/// Represents a postgres binary comparison operator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonOperator {
    pub operator_name: String,
    pub operator_kind: OperatorKind,
    pub argument_type: models::ScalarTypeName,
    pub is_infix: bool,
}

/// Is it a built-in operator, or a custom operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OperatorKind {
    Equal,
    In,
    Custom,
}

/// Mapping from a "table" name to its information.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TablesInfo(pub BTreeMap<models::CollectionName, TableInfo>);

impl TablesInfo {
    pub fn empty() -> Self {
        TablesInfo(BTreeMap::new())
    }

    pub fn merge(&mut self, other: TablesInfo) {
        for (table_name, table_info) in other.0 {
            self.0.insert(table_name, table_info);
        }
    }
}

/// Information about a database table (or any other kind of relation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    pub schema_name: String,
    pub table_name: String,
    pub columns: BTreeMap<models::FieldName, ColumnInfo>,

    pub uniqueness_constraints: UniquenessConstraints,

    pub foreign_relations: ForeignRelations,

    pub description: Option<String>,
}

/// Can this column contain null values
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Nullable {
    #[default]
    Nullable,
    NonNullable,
}

/// Does this column have a default value.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HasDefault {
    #[default]
    NoDefault,
    HasDefault,
}

/// Is this column an identity column.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum IsIdentity {
    #[default]
    NotIdentity,
    IdentityByDefault,
    IdentityAlways,
}

/// Is this column a generated column.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum IsGenerated {
    #[default]
    NotGenerated,
    Stored,
}

/// Information about a database column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInfo {
    pub name: String,
    pub r#type: Type,
    #[serde(default)]
    pub nullable: Nullable,
    #[serde(skip_serializing_if = "does_not_have_default")]
    #[serde(default)]
    pub has_default: HasDefault,
    #[serde(skip_serializing_if = "is_not_identity")]
    #[serde(default)]
    pub is_identity: IsIdentity,
    #[serde(skip_serializing_if = "is_not_generated")]
    #[serde(default)]
    pub is_generated: IsGenerated,
    #[serde(default)]
    pub description: Option<String>,
}

fn does_not_have_default(has_default: &HasDefault) -> bool {
    matches!(has_default, HasDefault::NoDefault)
}

fn is_not_identity(is_identity: &IsIdentity) -> bool {
    matches!(is_identity, IsIdentity::NotIdentity)
}

fn is_not_generated(is_generated: &IsGenerated) -> bool {
    matches!(is_generated, IsGenerated::NotGenerated)
}

/// A mapping from the name of a unique constraint to its value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UniquenessConstraints(pub BTreeMap<String, UniquenessConstraint>);

/// The set of columns that make up a uniqueness constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UniquenessConstraint(pub BTreeSet<models::FieldName>);

/// A mapping from the name of a foreign key constraint to its value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ForeignRelations(pub BTreeMap<String, ForeignRelation>);

/// A foreign key constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]

pub struct ForeignRelation {
    pub foreign_schema: Option<String>,
    pub foreign_table: String,
    pub column_mapping: BTreeMap<models::FieldName, models::FieldName>,
}

/// All supported aggregate functions, grouped by type.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct AggregateFunctions(
    pub BTreeMap<models::ScalarTypeName, BTreeMap<models::AggregateFunctionName, AggregateFunction>>,
);

impl AggregateFunctions {
    pub fn empty() -> Self {
        AggregateFunctions(BTreeMap::new())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AggregateFunction {
    pub return_type: models::TypeName,
}

/// Type representation of scalar types, grouped by type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TypeRepresentations(pub BTreeMap<models::ScalarTypeName, BigQueryType>);

impl TypeRepresentations {
    pub fn empty() -> Self {
        TypeRepresentations(BTreeMap::new())
    }
}

/// Type representation of a scalar type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BigQueryType {
    Array(Box<BigQueryType>),
    BigNumeric,
    Boolean,
    Bytes,
    Date,
    Datetime,
    Float64,
    Geography,
    Int64,
    Json,
    Numeric,
    Range(Box<TypeRange>),
    String,
    Struct(BTreeMap<String, Box<BigQueryType>>),
    Time,
    Timestamp,
}

impl From<BigQueryType> for models::TypeName {
    fn from(val: BigQueryType) -> Self {
        match val {
            BigQueryType::Array(_) => models::TypeName::new(SmolStr::new("array")),
            BigQueryType::BigNumeric => models::TypeName::new(SmolStr::new("bignumeric")),
            BigQueryType::Boolean => models::TypeName::new(SmolStr::new("boolean")),
            BigQueryType::Bytes => models::TypeName::new(SmolStr::new("bytes")),
            BigQueryType::Date => models::TypeName::new(SmolStr::new("date")),
            BigQueryType::Datetime => models::TypeName::new(SmolStr::new("datetime")),
            BigQueryType::Float64 => models::TypeName::new(SmolStr::new("float64")),
            BigQueryType::Geography => models::TypeName::new(SmolStr::new("geography")),
            BigQueryType::Int64 => models::TypeName::new(SmolStr::new("int64")),
            BigQueryType::Json => models::TypeName::new(SmolStr::new("json")),
            BigQueryType::Numeric => models::TypeName::new(SmolStr::new("numeric")),
            BigQueryType::Range(_) => models::TypeName::new(SmolStr::new("range")),
            BigQueryType::String => models::TypeName::new(SmolStr::new("string")),
            BigQueryType::Struct(_) => models::TypeName::new(SmolStr::new("struct")),
            BigQueryType::Time => models::TypeName::new(SmolStr::new("time")),
            BigQueryType::Timestamp => models::TypeName::new(SmolStr::new("timestamp")),
        }
    }
}
