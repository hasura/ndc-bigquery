// copy-pasted from
// https://github.com/hasura/v3-experiments/blob/gdc-spec/crates/gdc-client/src/models.rs

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

// ANCHOR: QueryRequest
/// This is the request body of the query POST endpoint
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QueryRequest {
    /// The name of a table
    pub table: String,
    /// The query syntax tree
    pub query: Query,
    /// Values to be provided to any table arguments
    pub arguments: HashMap<String, Argument>,
    /// Any relationships between tables involved in the query request
    pub table_relationships: HashMap<String, Relationship>,
    /// One set of named variables for each rowset to fetch. Each variable set
    /// should be subtituted in turn, and a fresh set of rows returned.
    pub variables: Option<Vec<HashMap<String, serde_json::Value>>>,
}
// ANCHOR_END: QueryRequest

// ANCHOR: Argument
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Argument {
    /// The argument is provided by reference to a variable
    Variable { name: String },
    /// The argument is provided as a literal value
    Literal { value: serde_json::Value },
}
// ANCHOR_END: Argument

// ANCHOR: Query
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Query {
    /// Aggregate fields of the query
    pub aggregates: Option<HashMap<String, Aggregate>>,
    /// Fields of the query
    pub fields: Option<HashMap<String, Field>>,
    /// Optionally limit to N results
    pub limit: Option<u32>,
    /// Optionally offset from the Nth result
    pub offset: Option<u32>,
    pub order_by: Option<OrderBy>,
    #[serde(rename = "where")]
    pub predicate: Option<Expression>,
}
// ANCHOR_END: Query

// ANCHOR: Aggregate
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Aggregate {
    // TODO: do we need aggregation row limits?
    ColumnCount {
        /// The column to apply the count aggregate function to
        column: String,
        /// Whether or not only distinct items should be counted
        distinct: bool,
    },
    SingleColumn {
        /// The column to apply the aggregation function to
        column: String,
        /// Single column aggregate function name.
        function: String,
    },
    StarCount {},
}
// ANCHOR_END: Aggregate

// ANCHOR: Field
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Field {
    Column {
        column: String,
        /// Values to be provided to any field arguments
        arguments: HashMap<String, Argument>,
    },
    Relationship {
        query: Box<Query>,
        /// The name of the relationship to follow for the subquery
        relationship: String,
        /// Values to be provided to any table arguments
        arguments: HashMap<String, Argument>,
    },
}
// ANCHOR_END: Field

// ANCHOR: OrderBy
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OrderBy {
    /// The elements to order by, in priority order
    pub elements: Vec<OrderByElement>,
}
// ANCHOR_END: OrderBy

// ANCHOR: OrderByElement
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OrderByElement {
    pub order_direction: OrderDirection,
    pub target: OrderByTarget,
}
// ANCHOR_END: OrderByElement

// ANCHOR: OrderByTarget
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrderByTarget {
    Column {
        /// The name of the column
        name: String,
        /// Any relationships to traverse to reach this column
        path: Vec<PathElement>,
    },
    SingleColumnAggregate {
        /// The column to apply the aggregation function to
        column: String,
        /// Single column aggregate function name.
        function: String,
        /// Non-empty collection of relationships to traverse
        path: Vec<PathElementWithPredicate>,
    },
    StarCountAggregate {
        /// Non-empty collection of relationships to traverse
        path: Vec<PathElementWithPredicate>,
    },
}
// ANCHOR_END: OrderByTarget

// ANCHOR: PathElementWithPredicate
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PathElementWithPredicate {
    /// The name of the relationship to follow
    pub relationship: String,
    /// Values to be provided to any table arguments
    pub arguments: HashMap<String, Argument>,
    /// A predicate expression to apply to the target table
    pub predicate: Box<Expression>,
}
// ANCHOR_END: PathElementWithPredicate

// ANCHOR: OrderDirection
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum OrderDirection {
    Asc,
    Desc,
}
// ANCHOR_END: OrderDirection

// ANCHOR: Expression
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Expression {
    And {
        expressions: Vec<Expression>,
    },
    Or {
        expressions: Vec<Expression>,
    },
    Not {
        expression: Box<Expression>,
    },
    UnaryComparisonOperator {
        column: Box<ComparisonTarget>,
        operator: Box<UnaryComparisonOperator>,
    },
    BinaryComparisonOperator {
        column: Box<ComparisonTarget>,
        operator: Box<BinaryComparisonOperator>,
        value: Box<ComparisonValue>,
    },
    BinaryArrayComparisonOperator {
        column: Box<ComparisonTarget>,
        operator: Box<BinaryArrayComparisonOperator>,
        values: Vec<ComparisonValue>,
    },
    Exists {
        in_table: Box<ExistsInTable>,
        #[serde(rename = "where")]
        predicate: Box<Expression>,
    },
}
// ANCHOR_END: Expression

// ANCHOR: UnaryComparisonOperator
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum UnaryComparisonOperator {
    IsNull,
}
// ANCHOR_END: UnaryComparisonOperator

// ANCHOR: BinaryArrayComparisonOperator
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum BinaryArrayComparisonOperator {
    In,
}
// ANCHOR_END: BinaryArrayComparisonOperator

// ANCHOR: BinaryComparisonOperator
#[derive(
    Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BinaryComparisonOperator {
    Equal,
    // should we rename this? To what?
    Other { name: String },
}
// ANCHOR_END: BinaryComparisonOperator

// ANCHOR: ComparisonTarget
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ComparisonTarget {
    Column {
        /// The name of the column
        name: String,
        /// Any relationships to traverse to reach this column
        path: Vec<PathElement>,
    },
    RootTableColumn {
        /// The name of the column
        name: String,
    },
}
// ANCHOR_END: ComparisonTarget

// ANCHOR: PathElement
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PathElement {
    /// The name of the relationship to follow
    pub relationship: String,
    /// Values to be provided to any table arguments
    pub arguments: HashMap<String, Argument>,
}
// ANCHOR_END: PathElement

// ANCHOR: ComparisonValue
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ComparisonValue {
    Column { column: Box<ComparisonTarget> },
    Scalar { value: serde_json::Value },
    Variable { name: String },
}
// ANCHOR_END: ComparisonValue

// ANCHOR: ExistsInTable
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExistsInTable {
    Related {
        relationship: String,
        /// Values to be provided to any table arguments
        arguments: HashMap<String, Argument>,
    },
    Unrelated {
        /// The name of a table
        table: String,
        /// Values to be provided to any table arguments
        arguments: HashMap<String, Argument>,
    },
}
// ANCHOR_END: ExistsInTable

// ANCHOR: Relationship
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Relationship {
    /// A mapping between columns on the source table to columns on the target table
    pub column_mapping: HashMap<String, String>,
    pub relationship_type: RelationshipType,
    /// The name of the table or object type which is the source of this relationship
    pub source_table_or_type: String,
    /// The name of a table
    pub target_table: String,
    /// Values to be provided to any table arguments
    pub arguments: HashMap<String, ComparisonValue>,
}
// ANCHOR_END: Relationship

// ANCHOR: RelationshipType
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    Object,
    Array,
}
// ANCHOR_END: RelationshipType
