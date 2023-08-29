//! Metadata information regarding the database and tracked information.

use std::collections::{BTreeMap, BTreeSet};

use enum_iterator::Sequence;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The scalar types supported by the Engine.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Sequence, Serialize, Deserialize, JsonSchema,
)]
pub enum ScalarType {
    Boolean,
    Float,
    Int,
    String,
    /// `Any` isn't supported by the engine.
    #[serde(rename = "any")]
    Any,
}

impl ScalarType {
    const OPERATORS_SUPPORTED_BY_ALL_TYPES: &[BinaryOperator] = &[
        BinaryOperator::Equals,
        BinaryOperator::NotEquals,
        BinaryOperator::LessThan,
        BinaryOperator::LessThanOrEqualTo,
        BinaryOperator::GreaterThan,
        BinaryOperator::GreaterThanOrEqualTo,
    ];

    const STRING_OPERATORS: &[BinaryOperator] = &[
        BinaryOperator::Like,
        BinaryOperator::NotLike,
        BinaryOperator::CaseInsensitiveLike,
        BinaryOperator::NotCaseInsensitiveLike,
        BinaryOperator::Similar,
        BinaryOperator::NotSimilar,
        BinaryOperator::Regex,
        BinaryOperator::NotRegex,
        BinaryOperator::CaseInsensitiveRegex,
        BinaryOperator::NotCaseInsensitiveRegex,
    ];

    /// Returns the complete set of comparison operators for the given type.
    pub fn comparison_operators(&self) -> BTreeSet<BinaryOperator> {
        let mut operators =
            BTreeSet::from_iter(Self::OPERATORS_SUPPORTED_BY_ALL_TYPES.iter().copied());
        operators.extend(match self {
            ScalarType::String => Self::STRING_OPERATORS.iter(),
            _ => [].iter(),
        });
        operators
    }
}

impl ToString for ScalarType {
    fn to_string(&self) -> String {
        match self {
            Self::Boolean => "Boolean".to_string(),
            Self::Float => "Float".to_string(),
            Self::Int => "Int".to_string(),
            Self::String => "String".to_string(),
            Self::Any => "any".to_string(),
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
pub enum BinaryOperator {
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

impl BinaryOperator {
    /// The name of the binary operator exposed via the schema.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Equals => "eq",
            Self::NotEquals => "neq",
            Self::LessThan => "lt",
            Self::LessThanOrEqualTo => "lte",
            Self::GreaterThan => "gt",
            Self::GreaterThanOrEqualTo => "gte",
            Self::Like => "like",
            Self::NotLike => "nlike",
            Self::CaseInsensitiveLike => "ilike",
            Self::NotCaseInsensitiveLike => "nilike",
            Self::Similar => "similar",
            Self::NotSimilar => "nsimilar",
            Self::Regex => "regex",
            Self::NotRegex => "nregex",
            Self::CaseInsensitiveRegex => "iregex",
            Self::NotCaseInsensitiveRegex => "niregex",
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

impl ToString for BinaryOperator {
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

/// Information about a database column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ColumnInfo {
    pub name: String,
    pub r#type: ScalarType,
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
            .collect::<BTreeSet<BinaryOperator>>();

        for operator in enum_iterator::all::<BinaryOperator>() {
            assert!(
                exposed_operators.contains(&operator),
                "The operator {:?} is not exposed anywhere.",
                operator
            );
        }
    }
}
