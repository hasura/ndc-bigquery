/// Type definitions of a SQL AST representation.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct With {
    pub recursive: bool,
    pub common_table_expressions: Vec<CommonTableExpression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonTableExpression {
    pub table_name: TableAlias,
    pub column_names: Option<Vec<ColumnAlias>>,
    pub select: Box<Select>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Select {
    pub with: With,
    pub select_list: SelectList,
    pub from: Option<From>,
    pub where_: Where,
    pub group_by: GroupBy,
    pub order_by: OrderBy,
    pub limit: Limit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectList(pub Vec<(ColumnAlias, Expression)>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum From {
    Table {
        name: TableName,
        alias: TableAlias,
    },
    Select {
        select: Box<Select>,
        alias: TableAlias,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Where(pub Expression);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupBy {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderBy {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limit {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    And {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Or {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Not(Box<Expression>),
    BinaryOperator {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    BinaryArrayOperator {
        left: Box<Expression>,
        operator: BinaryArrayOperator,
        right: Vec<Expression>,
    },
    FunctionCall {
        function: Function,
        args: Vec<Expression>,
    },
    // select queries can appear in a select list if they return
    // one row. For now we can only do this with 'row_to_json'.
    // Consider changing this if we encounter more ways.
    RowToJson(TableName),
    ColumnName(ColumnName),
    Value(Value),
}

// we should consider at least the list in `Hasura.Backends.Postgres.Translate.BoolExp`
// have skipped column checks for now, ie, CEQ, CNE, CGT etc
// have skipped casts for now
// we'd like to remove all the Not variants internally, but first we'll check there are no
// performance implications
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOperator {
    Equals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqualTo,
    LessThanOrEqualTo,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryArrayOperator {
    In,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Function {
    Coalesce,
    JsonAgg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Int4(i32),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
}

/// aliases that we give to relations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableAlias {
    pub unique_index: u64,
    pub name: String,
}
/// aliases that we give to columns
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnAlias {
    pub unique_index: u64,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableName {
    /// refers to a db table object name
    DBTable { schema: String, table: String },
    /// refers to an alias we created
    AliasedTable(TableAlias),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnName {
    /// refers to a db column object name
    TableColumn { table: TableName, name: String },
    /// refers to an alias we created
    AliasedColumn {
        table: TableName,
        alias: ColumnAlias,
    },
}

// utils
pub fn empty_with() -> With {
    With {
        recursive: false,
        common_table_expressions: vec![],
    }
}
pub fn empty_where() -> Expression {
    Expression::Value(Value::Bool(true))
}
pub fn empty_group_by() -> GroupBy {
    GroupBy {}
}
pub fn empty_order_by() -> OrderBy {
    OrderBy {}
}
pub fn empty_limit() -> Limit {
    Limit {
        limit: None,
        offset: None,
    }
}

pub fn true_expr() -> Expression {
    Expression::Value(Value::Bool(true))
}
pub fn false_expr() -> Expression {
    Expression::Value(Value::Bool(false))
}

impl TableName {
    pub fn from_public(tablename: String) -> TableName {
        TableName::DBTable {
            schema: "public".to_string(),
            table: tablename,
        }
    }
}

/// create a simple select with a select list and the rest are empty.
pub fn simple_select(select_list: Vec<(ColumnAlias, Expression)>) -> Select {
    Select {
        with: empty_with(),
        select_list: SelectList(select_list),
        from: None,
        where_: Where(empty_where()),
        group_by: empty_group_by(),
        order_by: empty_order_by(),
        limit: empty_limit(),
    }
}

/// wrap an existing select in row_to_json and json_agg
pub fn select_as_json(
    select: Select,
    column_alias: ColumnAlias,
    table_alias: TableAlias,
) -> Select {
    Select {
        with: empty_with(),
        // This translates to: `coalesce(json_agg(row_to_json(<table_alias>)), '[]') AS <column_alias>`.
        //
        // - `row_to_json` takes a row and converts it to a json object.
        // - `json_agg` aggregates the json objects to a json array.
        // - `coalesce(<thing>, <otherwise>)` returns <thing> if it is not null, and <otherwise> if it is null.
        select_list: SelectList(vec![(
            column_alias,
            Expression::FunctionCall {
                function: Function::Coalesce,
                args: vec![
                    Expression::FunctionCall {
                        function: Function::JsonAgg,
                        args: vec![Expression::RowToJson(TableName::AliasedTable(
                            table_alias.clone(),
                        ))],
                    },
                    Expression::Value(Value::String("[]".to_string())),
                ],
            },
        )]),
        // FROM (select ...) as <table_alias>
        from: Some(From::Select {
            select: Box::new(select),
            alias: table_alias,
        }),
        where_: Where(empty_where()),
        group_by: empty_group_by(),
        order_by: empty_order_by(),
        limit: empty_limit(),
    }
}
