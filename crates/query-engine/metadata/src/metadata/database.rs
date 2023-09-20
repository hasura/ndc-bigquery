//! Metadata information regarding the database and tracked information.

use std::collections::{BTreeMap, BTreeSet};

use enum_iterator::Sequence;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The scalar types supported by the Engine.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum ScalarType {
    Boolean,
    Smallint,
    Integer,
    Bigint,
    Real,
    #[serde(rename = "double precision")]
    DoublePrecision,
    Numeric,
    Character,
    #[serde(rename = "character varying")]
    CharacterVarying,
    Text,
    Json,
    Jsonb,
    Date,
    #[serde(rename = "time with time zone")]
    TimeWithTimeZone,
    #[serde(rename = "time without time zone")]
    TimeWithoutTimeZone,
    #[serde(rename = "timestamp with time zone")]
    TimestampWithTimeZone,
    #[serde(rename = "timestamp without time zone")]
    TimestampWithoutTimeZone,
    Uuid,
    Any,
}

impl ScalarType {
    const OPERATORS_SUPPORTED_BY_ALL_TYPES: &[ComparisonOperator] = &[
        ComparisonOperator::Equals,
        ComparisonOperator::NotEquals,
        ComparisonOperator::LessThan,
        ComparisonOperator::LessThanOrEqualTo,
        ComparisonOperator::GreaterThan,
        ComparisonOperator::GreaterThanOrEqualTo,
    ];

    const STRING_OPERATORS: &[ComparisonOperator] = &[
        ComparisonOperator::Like,
        ComparisonOperator::NotLike,
        ComparisonOperator::CaseInsensitiveLike,
        ComparisonOperator::NotCaseInsensitiveLike,
        ComparisonOperator::Similar,
        ComparisonOperator::NotSimilar,
        ComparisonOperator::Regex,
        ComparisonOperator::NotRegex,
        ComparisonOperator::CaseInsensitiveRegex,
        ComparisonOperator::NotCaseInsensitiveRegex,
    ];

    /// Returns the complete set of comparison operators for the given type.
    pub fn comparison_operators(&self) -> BTreeSet<ComparisonOperator> {
        let mut operators =
            BTreeSet::from_iter(Self::OPERATORS_SUPPORTED_BY_ALL_TYPES.iter().copied());
        operators.extend(match self {
            ScalarType::CharacterVarying => Self::STRING_OPERATORS.iter(),
            ScalarType::Text => Self::STRING_OPERATORS.iter(),
            _ => [].iter(),
        });
        operators
    }
}

impl ToString for ScalarType {
    fn to_string(&self) -> String {
        match self {
            ScalarType::DoublePrecision => "double precision".to_string(),
            ScalarType::CharacterVarying => "character varying".to_string(),
            ScalarType::TimeWithTimeZone => "time with time zone".to_string(),
            ScalarType::TimeWithoutTimeZone => "time without time zone".to_string(),
            ScalarType::TimestampWithTimeZone => "timestamp with time zone".to_string(),
            ScalarType::TimestampWithoutTimeZone => "timestamp without time zone".to_string(),
            _ => format!("{:?}", self).to_lowercase(),
        }
    }
}

/// The complete list of supported binary operators for scalar types.
/// Not all of these are supported for every type.
///
/// These must be kept in sync with the documentation.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Sequence,
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEqualTo,
    GreaterThan,
    GreaterThanOrEqualTo,
    Like,
    NotLike,
    CaseInsensitiveLike,
    NotCaseInsensitiveLike,
    Similar,
    NotSimilar,
    Regex,
    NotRegex,
    CaseInsensitiveRegex,
    NotCaseInsensitiveRegex,
}

impl ComparisonOperator {
    /// The name of the binary operator exposed via the schema.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Equals => "_eq",
            Self::NotEquals => "_neq",
            Self::LessThan => "_lt",
            Self::LessThanOrEqualTo => "_lte",
            Self::GreaterThan => "_gt",
            Self::GreaterThanOrEqualTo => "_gte",
            Self::Like => "_like",
            Self::NotLike => "_nlike",
            Self::CaseInsensitiveLike => "_ilike",
            Self::NotCaseInsensitiveLike => "_nilike",
            Self::Similar => "_similar",
            Self::NotSimilar => "_nsimilar",
            Self::Regex => "_regex",
            Self::NotRegex => "_nregex",
            Self::CaseInsensitiveRegex => "_iregex",
            Self::NotCaseInsensitiveRegex => "_niregex",
        }
    }

    /// Computes the argument type on the right-hand side of the operator,
    /// given the type of the value on the left-hand side.
    ///
    /// In practice, operators are always assumed to be operating on two values
    /// of the same type, so this just returns its input.
    pub fn rhs_argument_type(&self, lhs: ScalarType) -> ScalarType {
        lhs
    }
}

impl ToString for ComparisonOperator {
    fn to_string(&self) -> String {
        self.name().to_string()
    }
}

/// Mapping from a "table" name to its information.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct TablesInfo(pub BTreeMap<String, TableInfo>);

/// Information about a database table (or any other kind of relation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TableInfo {
    pub schema_name: String,
    pub table_name: String,
    pub columns: BTreeMap<String, ColumnInfo>,
    #[serde(default)]
    pub uniqueness_constraints: UniquenessConstraints,
    #[serde(default)]
    pub foreign_relations: ForeignRelations,
}

/// Can this column contain null values
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub enum Nullable {
    #[default]
    Nullable,
    NonNullable,
}

/// Information about a database column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ColumnInfo {
    pub name: String,
    pub r#type: ScalarType,
    #[serde(default)]
    pub nullable: Nullable,
}

/// A mapping from the name of a unique constraint to its value.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct UniquenessConstraints(pub BTreeMap<String, UniquenessConstraint>);

/// The set of columns that make up a uniqueness constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UniquenessConstraint(pub BTreeSet<String>);

/// A mapping from the name of a foreign key constraint to its value.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ForeignRelations(pub BTreeMap<String, ForeignRelation>);

/// A foreign key constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ForeignRelation {
    pub foreign_table: String,
    pub column_mapping: BTreeMap<String, String>,
}

/// All supported aggregate functions, grouped by type.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct AggregateFunctions(pub BTreeMap<ScalarType, BTreeMap<String, AggregateFunction>>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AggregateFunction {
    pub return_type: ScalarType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_all_binary_comparison_operators_are_used() {
        // This is the set of all operators exposed through the schema.
        let exposed_operators = enum_iterator::all::<ScalarType>()
            .flat_map(|scalar_type| scalar_type.comparison_operators())
            .collect::<BTreeSet<ComparisonOperator>>();

        for operator in enum_iterator::all::<ComparisonOperator>() {
            assert!(
                exposed_operators.contains(&operator),
                "The operator {:?} is not exposed anywhere.",
                operator
            );
        }
    }
}
