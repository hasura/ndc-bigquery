//! Type definitions of a SQL AST representation.

use std::collections::BTreeMap;

/// An EXPLAIN clause
#[derive(Debug, Clone, PartialEq)]
pub enum Explain<'a> {
    Select(&'a Select),
}

/// A WITH clause
#[derive(Debug, Clone, PartialEq)]
pub struct With {
    pub common_table_expressions: Vec<CommonTableExpression>,
}

/// A single Common Table Expression
#[derive(Debug, Clone, PartialEq)]
pub struct CommonTableExpression {
    pub alias: TableAlias,
    pub column_names: Option<Vec<ColumnAlias>>,
    pub select: CTExpr,
}

/// The 'body' side of a Common Table Expression
#[derive(Debug, Clone, PartialEq)]
pub enum CTExpr {
    Select(Select),
    RawSql(Vec<RawSql>),
    // Delete(Delete),
    // Insert(Insert),
    // Update(Update),
}

/// Raw SQL written by a user which is opaque to us
#[derive(Debug, Clone, PartialEq)]
pub enum RawSql {
    /// Raw SQL text
    RawText(String),
    /// An expression
    Expression(Expression),
}

/// A SELECT clause
#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    pub with: With,
    pub select_list: SelectList,
    pub from: Option<From>,
    pub joins: Vec<Join>,
    pub where_: Where,
    pub group_by: GroupBy,
    pub order_by: OrderBy,
    pub limit: Limit,
}

/// An INSERT clause
#[derive(Debug, Clone, PartialEq)]
pub struct Insert {
    pub schema: SchemaName,
    pub table: TableName,
    pub columns: Option<Vec<ColumnName>>,
    pub from: InsertFrom,
    pub returning: Returning,
}

/// Source from which values would be inserted.
#[derive(Debug, Clone, PartialEq)]
pub enum InsertFrom {
    Values(Vec<Vec<MutationValueExpression>>),
    Select(Select),
}

/// An expression inside an INSERT VALUES clause or UPDATE SET clause.
#[derive(Debug, Clone, PartialEq)]
pub enum MutationValueExpression {
    Default,
    Expression(Expression),
}

/// A DELETE clause
#[derive(Debug, Clone, PartialEq)]
pub struct Delete {
    pub from: From,
    pub where_: Where,
    pub returning: Returning,
}

/// An UPDATE clause
#[derive(Debug, Clone, PartialEq)]
pub struct Update {
    pub schema: SchemaName,
    pub table: TableName,
    pub set: BTreeMap<ColumnName, MutationValueExpression>,
    pub where_: Where,
    pub returning: Returning,
}

/// a RETURNING clause
#[derive(Debug, Clone, PartialEq)]
pub struct Returning(pub SelectList);

/// A select list
#[derive(Debug, Clone, PartialEq)]
pub enum SelectList {
    SelectList(Vec<(ColumnAlias, Expression)>),
    SelectStar,
    SelectStarFrom(TableReference),
    Select1,
    SelectStarComposite(Expression),
    SelectListComposite(Box<SelectList>, Box<SelectList>),
}

/// A FROM clause
#[derive(Debug, Clone, PartialEq)]
pub enum From {
    /// Select from a table reference
    Table {
        reference: TableReference,
        alias: TableAlias,
    },
    /// Select from a subquery
    Select {
        select: Box<Select>,
        alias: TableAlias,
    },
    /// Convert a json array of objects to a relation.
    /// Should probably be of the form `jsonb_to_recordset(cast($1 as json))`
    JsonbToRecordset {
        expression: Expression,
        alias: TableAlias,
        columns: Vec<(ColumnAlias, ScalarType)>,
    },
    JsonbArrayElements {
        expression: Expression,
        alias: TableAlias,
        column: ColumnAlias,
    },
    Unnest {
        expression: Expression,
        alias: TableAlias,
        column: ColumnAlias,
    },
}

/// A JOIN clause
#[derive(Debug, Clone, PartialEq)]
pub enum Join {
    /// LEFT OUTER JOIN
    LeftOuterJoin(LeftOuterJoin),
    /// INNER JOIN LATERAL
    InnerJoin(InnerJoin),
    /// CROSS JOIN
    CrossJoin(CrossJoin),
    /// FULL OUTER JOIN
    FullOuterJoin(FullOuterJoin),
}

// todo(PY): lateral joins?
impl Join {
    /// Get the select expression and table alias regardless of the join type.
    pub fn get_select_and_alias(self) -> (Box<Select>, TableAlias) {
        match self {
            Join::LeftOuterJoin(LeftOuterJoin { select, alias, .. }) => (select, alias),
            Join::InnerJoin(InnerJoin { select, alias }) => (select, alias),
            Join::CrossJoin(CrossJoin { select, alias }) => (select, alias),
            Join::FullOuterJoin(FullOuterJoin { select, alias }) => (select, alias),
        }
    }
}

/// A CROSS JOIN clause
#[derive(Debug, Clone, PartialEq)]
pub struct CrossJoin {
    pub select: Box<Select>,
    pub alias: TableAlias,
}

/// A LEFT OUTER JOIN clause
#[derive(Debug, Clone, PartialEq)]
pub struct LeftOuterJoin {
    pub select: Box<Select>,
    pub alias: TableAlias,
    pub on: Expression,
}

/// An INNER JOIN clause
#[derive(Debug, Clone, PartialEq)]
pub struct InnerJoin {
    pub select: Box<Select>,
    pub alias: TableAlias,
}

/// A FULL OUTER JOIN LATERAL clause
#[derive(Debug, Clone, PartialEq)]
pub struct FullOuterJoin {
    pub select: Box<Select>,
    pub alias: TableAlias,
}

/// A WHERE clause
#[derive(Debug, Clone, PartialEq)]
pub struct Where(pub Expression);

/// A GROUP BY clause, currently not in use
#[derive(Debug, Clone, PartialEq)]
pub struct GroupBy {}

/// An ORDER BY clause
#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    pub elements: Vec<OrderByElement>,
}

// todo: should we also include option for specifying NULLS FIRST | NULLS LAST
/// A single element in an ORDER BY clause
#[derive(Debug, Clone, PartialEq)]
pub struct OrderByElement {
    pub target: Expression,
    pub direction: OrderByDirection,
}

/// A direction for a single ORDER BY element
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderByDirection {
    Asc,
    Desc,
}

/// LIMIT and OFFSET clauses
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limit {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// A scalar expression
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// AND clause
    And {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// OR clause
    Or {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    /// NOT clause
    Not(Box<Expression>),
    /// A binary operation on two scalar expression
    BinaryOperation {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    /// A binary operation on a scalar expression and an array of scalar expressions
    BinaryArrayOperation {
        left: Box<Expression>,
        operator: BinaryArrayOperator,
        right: Vec<Expression>,
    },
    /// An unary operation on a scalar expression
    UnaryOperation {
        expression: Box<Expression>,
        operator: UnaryOperator,
    },
    /// A scalar function call
    FunctionCall {
        function: Function,
        args: Vec<Expression>,
    },
    /// An EXISTS clause
    Exists {
        select: Box<Select>,
    },
    /// A json_build_object function call
    JsonBuildObject(BTreeMap<String, Expression>),
    // SELECT queries can appear in a select list if they return
    // one row. For now we can only do this with 'row_to_json'.
    // Consider changing this if we encounter more ways.
    /// A row_to_json function call
    RowToJson(TableReference),
    /// A column reference
    ColumnReference(ColumnReference),
    /// A table reference
    TableReference(TableReference),
    /// An irreducible value
    Value(Value),
    Cast {
        expression: Box<Expression>,
        r#type: ScalarType,
    },
    /// A COUNT clause
    Count(CountType),
    ArrayConstructor(Vec<Expression>),
    CorrelatedSubSelect(Box<Select>),
    NestedFieldSelect {
        expression: Box<Expression>,
        nested_field: NestedField,
    },
    // JsonQuery(Box<Expression>, JsonPath), // JSON_QUERY([album].[json], '$.title') for multiple
    // // values
    // JsonValue(Box<Expression>, JsonPath), // JSON_VALUE([album].[json], '$.title') for single values
}

// // JSON selector path for expressing '$.user.name'
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub struct JsonPath {
//     pub elements: Vec<ColumnAlias>,
// }

/// Represents the name of a field in a nested object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedField(pub String);

/// An unary operator
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOperator {
    IsNull,
}

/// Represents the name of a binary operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryOperator(pub String);

/// A binary operator when the rhs is an array
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryArrayOperator {
    In,
}

/// A scalar function
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Function {
    Coalesce,
    JsonAgg,
    JsonbPopulateRecord,
    ArrayAgg,
    Unnest,
    Unknown(String),
}

/// COUNT clause
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CountType {
    Star,
    Simple(ColumnReference),
    Distinct(ColumnReference),
}

/// Value
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int8(i32),
    Float8(f64),
    Bool(bool),
    Character(String),
    String(String),
    Null,
    Array(Vec<Value>),
    JsonValue(serde_json::Value),
    EmptyJsonArray,
    Variable(String),
}

/// Scalar type
#[derive(Debug, Clone, PartialEq)]

pub enum ScalarType {
    BaseType(ScalarTypeName),
    ArrayType(ScalarTypeName),
}

/// Scalar type name. This will always be output as a quoted identifier.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarTypeName {
    /// A type name referencing a schema.
    Qualified {
        schema_name: SchemaName,
        type_name: String,
    },
    /// A type name without a schema.
    Unqualified(String),
}

/// A database schema name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SchemaName(pub String);

/// A database table name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableName(pub String);

/// A reference to a table. Used when we want to query it,
/// for example in a FROM clause.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TableReference {
    /// refers to a db table object name
    DBTable {
        schema: SchemaName,
        table: TableName,
    },
    /// refers to an alias we created
    AliasedTable(TableAlias),
}

/// A database table's column name
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColumnName(pub String);

/// A reference to a column. Used when we want to query it,
/// for example in a SELECT list.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColumnReference {
    /// refers to a db column object name
    TableColumn {
        table: TableReference,
        name: ColumnName,
    },
    /// refers to an alias we created
    AliasedColumn {
        table: TableReference,
        column: ColumnAlias,
    },
}

/// aliases that we give to relations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableAlias {
    pub unique_index: u64,
    pub name: String,
}

/// aliases that we give to columns
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColumnAlias {
    pub name: String,
}
