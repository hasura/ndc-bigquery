/// Convert a SQL AST to a low-level SQL string.
use super::sql_ast::*;
use super::sql_string::*;

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
