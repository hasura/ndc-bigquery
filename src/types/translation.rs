#[derive(Debug)]
pub struct ExecutionPlan {
    pub root_field: String,
    pub pre: Vec<DDL>,
    pub query: Select,
    pub post: Vec<DDL>,
}
#[derive(Debug)]
pub struct SQL {
    pub sql: String,
    pub params: Vec<Value>,
    param_index: u64,
}
#[derive(Debug)]
pub struct DDL(pub SQL);

impl ExecutionPlan {
    pub fn query(&self) -> SQL {
        let mut sql = SQL::new();
        self.query.to_sql(&mut sql);
        sql
    }
}

impl SQL {
    pub fn new() -> SQL {
        SQL {
            sql: "".to_string(),
            params: vec![],
            param_index: 0,
        }
    }
    pub fn append_syntax(&mut self, sql: &str) {
        self.sql.push_str(sql);
    }
    pub fn append_identifier(&mut self, sql: &String) {
        // todo: sanitize
        self.sql.push_str(format!("\"{}\"", sql).as_str());
    }
    pub fn append_param(&mut self, value: Value) {
        self.param_index += 1;
        self.sql.push_str(format!("${}", self.param_index).as_str());
        self.params.push(value);
    }
}

// SQL AST

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

// Convert to SQL strings

impl With {
    pub fn to_sql(&self, sql: &mut SQL) {
        if self.common_table_expressions.is_empty() {
        } else {
            sql.append_syntax("WITH ");
            if self.recursive {
                sql.append_syntax("RECURSIVE ");
            }
            for cte in &self.common_table_expressions {
                sql.append_syntax(" ");
                cte.to_sql(sql);
                sql.append_syntax(",");
            }
            sql.sql.pop(); // removing the last comma added by cte
        }
    }
}

impl CommonTableExpression {
    pub fn to_sql(&self, sql: &mut SQL) {
        self.table_name.to_sql(sql);
        match &self.column_names {
            None => {}
            Some(names) => {
                sql.append_syntax("(");
                for name in names {
                    name.to_sql(sql);
                }
                sql.append_syntax(")");
            }
        }

        sql.append_syntax(" AS (");
        self.select.to_sql(sql);
        sql.append_syntax(")");
    }
}

impl Select {
    pub fn to_sql(&self, sql: &mut SQL) {
        sql.append_syntax("SELECT ");
        for (col, expr) in &self.select_list {
            expr.to_sql(sql);
            sql.append_syntax(" AS ");
            col.to_sql(sql);
            sql.append_syntax(",");
        }
        sql.sql.pop(); // pop the last comma
        sql.append_syntax(" ");

        self.from.to_sql(sql);
    }
}

impl From {
    pub fn to_sql(&self, sql: &mut SQL) {
        sql.append_syntax("FROM ");
        match &self {
            From::Table { name, alias } => {
                name.to_sql(sql);
                sql.append_syntax(" AS ");
                alias.to_sql(sql);
            }
        }
    }
}

// scalars
impl Expression {
    pub fn to_sql(&self, sql: &mut SQL) {
        match &self {
            Expression::ColumnName(column_name) => column_name.to_sql(sql),
            Expression::Value(value) => value.to_sql(sql),
        }
    }
}
impl Value {
    pub fn to_sql(&self, sql: &mut SQL) {
        match &self {
            Value::Int4(i) => sql.append_syntax(format!("{}", i).as_str()),
            Value::Bool(true) => sql.append_syntax("true"),
            Value::Bool(false) => sql.append_syntax("false"),
        }
    }
}

// names
impl TableName {
    pub fn to_sql(&self, sql: &mut SQL) {
        match self {
            TableName::DBTable(name) => sql.append_identifier(&name.to_string()),
            TableName::AliasedTable(alias) => alias.to_sql(sql),
        };
    }
}

impl TableAlias {
    pub fn to_sql(&self, sql: &mut SQL) {
        let name = format!("hasu_tbl_{}_{}", self.unique_index, self.name);
        sql.append_identifier(&name);
    }
}

impl ColumnName {
    pub fn to_sql(&self, sql: &mut SQL) {
        match self {
            ColumnName::TableColumn(name) => sql.append_identifier(&name.to_string()),
            ColumnName::AliasedColumn(alias) => alias.to_sql(sql),
        };
    }
}

impl ColumnAlias {
    pub fn to_sql(&self, sql: &mut SQL) {
        let name = format!("hasu_col_{}_{}", self.unique_index, self.name);
        sql.append_identifier(&name);
    }
}
