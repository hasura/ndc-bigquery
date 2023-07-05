/// Type definitions of a SQL AST representation.

#[derive(Debug)]
pub struct With {
    pub recursive: bool,
    pub common_table_expressions: Vec<CommonTableExpression>,
}

#[derive(Debug)]
pub struct CommonTableExpression {
    pub table_name: TableAlias,
    pub column_names: Option<Vec<ColumnAlias>>,
    pub select: Box<Select>,
}

#[derive(Debug)]
pub struct Select {
    pub with: With,
    pub select_list: SelectList,
    pub from: From,
    pub where_: Where,
    pub group_by: GroupBy,
    pub order_by: OrderBy,
    pub limit: Limit,
}

#[derive(Debug)]
pub struct SelectList(pub Vec<(ColumnAlias, Expression)>);

#[derive(Debug)]
pub enum From {
    Table { name: TableName, alias: TableAlias },
}

#[derive(Debug)]
pub struct Where(pub Expression);

#[derive(Debug)]
pub struct GroupBy {}

#[derive(Debug)]
pub struct OrderBy {}

#[derive(Debug)]
pub struct Limit {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
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
    ColumnName(ColumnName),
    Value(Value),
}

#[derive(Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Equals,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Int4(i32),
    Bool(bool),
}

/// aliases that we give to relations
#[derive(Debug)]
pub struct TableAlias {
    pub unique_index: u64,
    pub name: String,
}
/// aliases that we give to columns
#[derive(Debug, PartialEq, Eq)]
pub struct ColumnAlias {
    pub unique_index: u64,
    pub name: String,
}
#[derive(Debug)]
pub enum TableName {
    /// refers to a db table object name
    DBTable { schema: String, table: String },
    /// refers to an alias we created
    AliasedTable(TableAlias),
}
#[derive(Debug, PartialEq, Eq)]
pub enum ColumnName {
    /// refers to a db column object name
    TableColumn { table: String, name: String },
    /// refers to an alias we created
    AliasedColumn { table: String, alias: ColumnAlias },
}

// utils
pub fn simple_select(select_list: Vec<(ColumnAlias, Expression)>, from: From) -> Select {
    Select {
        with: empty_with(),
        select_list: SelectList(select_list),
        from,
        where_: Where(empty_where()),
        group_by: empty_group_by(),
        order_by: empty_order_by(),
        limit: empty_limit(),
    }
}

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
