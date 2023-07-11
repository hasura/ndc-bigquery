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

            let ctes = &self.common_table_expressions;
            for (index, cte) in ctes.iter().enumerate() {
                cte.to_sql(sql);
                if index < (ctes.len() - 1) {
                    sql.append_syntax(", ")
                }
            }
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
        let SelectList(select_list) = &self.select_list;
        for (index, (col, expr)) in select_list.iter().enumerate() {
            expr.to_sql(sql);
            sql.append_syntax(" AS ");
            col.to_sql(sql);
            if index < (select_list.len() - 1) {
                sql.append_syntax(", ")
            }
        }

        sql.append_syntax(" ");

        match &self.from {
            Some(from) => from.to_sql(sql),
            None => (),
        }
        self.where_.to_sql(sql);
        self.limit.to_sql(sql);
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
            From::Select { select, alias } => {
                sql.append_syntax("(");
                select.to_sql(sql);
                sql.append_syntax(")");
                sql.append_syntax(" AS ");
                alias.to_sql(sql);
            }
        }
    }
}

impl Where {
    pub fn to_sql(&self, sql: &mut SQL) {
        let Where(expression) = self;
        if *expression != true_expr() {
            sql.append_syntax(" WHERE ");
            expression.to_sql(sql);
        }
    }
}

// scalars
impl Expression {
    pub fn to_sql(&self, sql: &mut SQL) {
        match &self {
            Expression::ColumnName(column_name) => column_name.to_sql(sql),
            Expression::Value(value) => value.to_sql(sql),
            Expression::And { left, right } => {
                sql.append_syntax("(");
                left.to_sql(sql);
                sql.append_syntax(" AND ");
                right.to_sql(sql);
                sql.append_syntax(")");
            }
            Expression::Or { left, right } => {
                sql.append_syntax("(");
                left.to_sql(sql);
                sql.append_syntax(" OR ");
                right.to_sql(sql);
                sql.append_syntax(")");
            }
            Expression::Not(expr) => {
                sql.append_syntax("NOT ");
                expr.to_sql(sql);
            }
            Expression::BinaryOperator {
                left,
                operator,
                right,
            } => {
                sql.append_syntax("(");
                left.to_sql(sql);
                operator.to_sql(sql);
                right.to_sql(sql);
                sql.append_syntax(")");
            }
            Expression::BinaryArrayOperator {
                left,
                operator,
                right,
            } => {
                sql.append_syntax("(");
                {
                    left.to_sql(sql);
                    operator.to_sql(sql);
                    sql.append_syntax("(");
                    for (index, item) in right.iter().enumerate() {
                        item.to_sql(sql);
                        if index < (right.len() - 1) {
                            sql.append_syntax(", ")
                        }
                    }
                    sql.append_syntax(")");
                }
                sql.append_syntax(")");
            }
            Expression::FunctionCall { function, args } => {
                function.to_sql(sql);
                sql.append_syntax("(");
                for (index, arg) in args.iter().enumerate() {
                    arg.to_sql(sql);
                    if index < (args.len() - 1) {
                        sql.append_syntax(", ")
                    }
                }
                sql.append_syntax(")");
            }
            Expression::RowToJson(select) => {
                sql.append_syntax("row_to_json");
                sql.append_syntax("(");
                select.to_sql(sql);
                sql.append_syntax(")");
            }
        }
    }
}

impl BinaryOperator {
    pub fn to_sql(&self, sql: &mut SQL) {
        match self {
            BinaryOperator::Equals => sql.append_syntax(" = "),
            BinaryOperator::GreaterThan => sql.append_syntax(" > "),
            BinaryOperator::GreaterThanOrEqualTo => sql.append_syntax(" >= "),
            BinaryOperator::LessThan => sql.append_syntax(" < "),
            BinaryOperator::LessThanOrEqualTo => sql.append_syntax(" <= "),
            BinaryOperator::Like => sql.append_syntax(" LIKE "),
            BinaryOperator::NotLike => sql.append_syntax(" NOT LIKE "),
            BinaryOperator::CaseInsensitiveLike => sql.append_syntax(" ILIKE "),
            BinaryOperator::NotCaseInsensitiveLike => sql.append_syntax(" NOT ILIKE "),
            BinaryOperator::Similar => sql.append_syntax(" SIMILAR TO "),
            BinaryOperator::NotSimilar => sql.append_syntax(" NOT SIMILAR TO "),
            BinaryOperator::Regex => sql.append_syntax(" ~ "),
            BinaryOperator::NotRegex => sql.append_syntax(" !~ "),
            BinaryOperator::CaseInsensitiveRegex => sql.append_syntax(" ~* "),
            BinaryOperator::NotCaseInsensitiveRegex => sql.append_syntax(" !~* "),
        }
    }
}

impl BinaryArrayOperator {
    pub fn to_sql(&self, sql: &mut SQL) {
        match self {
            BinaryArrayOperator::In => sql.append_syntax(" IN "),
        }
    }
}

impl Function {
    pub fn to_sql(&self, sql: &mut SQL) {
        match self {
            Function::Coalesce => sql.append_syntax("coalesce"),
            Function::JsonAgg => sql.append_syntax("json_agg"),
        }
    }
}

impl Value {
    pub fn to_sql(&self, sql: &mut SQL) {
        match &self {
            Value::Int4(i) => sql.append_syntax(format!("{}", i).as_str()),
            Value::String(s) => sql.append_syntax(format!("'{}'", s).as_str()),
            Value::Bool(true) => sql.append_syntax("true"),
            Value::Bool(false) => sql.append_syntax("false"),
            Value::Array(items) => {
                sql.append_syntax("ARRAY [");
                for (index, item) in items.iter().enumerate() {
                    item.to_sql(sql);
                    if index < (items.len() - 1) {
                        sql.append_syntax(", ")
                    }
                }
                sql.append_syntax("]");
            }
        }
    }
}

impl Limit {
    pub fn to_sql(&self, sql: &mut SQL) {
        match self.limit {
            None => (),
            Some(limit) => {
                sql.append_syntax(" LIMIT ");
                sql.append_syntax(format!("{}", limit).as_str());
            }
        };
        match self.offset {
            None => (),
            Some(offset) => {
                sql.append_syntax(" OFFSET ");
                sql.append_syntax(format!("{}", offset).as_str());
            }
        };
    }
}

// names
impl TableName {
    pub fn to_sql(&self, sql: &mut SQL) {
        match self {
            TableName::DBTable { schema, table } => {
                sql.append_identifier(schema);
                sql.append_syntax(".");
                sql.append_identifier(table);
            }
            TableName::AliasedTable(alias) => alias.to_sql(sql),
        };
    }
}

impl TableAlias {
    pub fn to_sql(&self, sql: &mut SQL) {
        //let name = format!("hasu_tbl_{}_{}", self.unique_index, self.name);
        let name = self.name.to_string();
        sql.append_identifier(&name);
    }
}

impl ColumnName {
    pub fn to_sql(&self, sql: &mut SQL) {
        match self {
            ColumnName::TableColumn { table, name } => {
                table.to_sql(sql);
                sql.append_syntax(".");
                sql.append_identifier(&name.to_string());
            }
            ColumnName::AliasedColumn { table, alias } => {
                table.to_sql(sql);
                sql.append_syntax(".");
                alias.to_sql(sql);
            }
        };
    }
}

impl ColumnAlias {
    pub fn to_sql(&self, sql: &mut SQL) {
        //let name = format!("hasu_col_{}_{}", self.unique_index, self.name);
        let name = self.name.to_string();
        sql.append_identifier(&name);
    }
}
