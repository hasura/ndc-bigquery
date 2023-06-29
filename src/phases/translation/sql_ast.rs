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
    pub select_list: Vec<(ColumnAlias, Expression)>,
    pub from: From,
    pub where_: Expression,
    pub group_by: GroupBy,
    pub order_by: OrderBy,
    pub limit: Limit,
}

#[derive(Debug)]
pub enum From {
    Table { name: TableName, alias: TableAlias },
}

#[derive(Debug)]
pub struct Where {}
#[derive(Debug)]
pub struct GroupBy {}
#[derive(Debug)]
pub struct OrderBy {}
#[derive(Debug)]
pub struct Limit {}

#[derive(Debug)]
pub enum Expression {
    ColumnName(ColumnName),
    Value(Value),
}
#[derive(Debug)]
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
#[derive(Debug)]
pub struct ColumnAlias {
    pub unique_index: u64,
    pub name: String,
}
#[derive(Debug)]
pub enum TableName {
    /// refers to a db table object name
    DBTable(String),
    /// refers to an alias we created
    AliasedTable(TableAlias),
}
#[derive(Debug)]
pub enum ColumnName {
    /// refers to a db column object name
    TableColumn(String),
    /// refers to an alias we created
    AliasedColumn(ColumnAlias),
}

// utils
pub fn simple_select(select_list: Vec<(ColumnAlias, Expression)>, from: From) -> Select {
    Select {
        with: empty_with(),
        select_list,
        from,
        where_: empty_where(),
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
    Limit {}
}
