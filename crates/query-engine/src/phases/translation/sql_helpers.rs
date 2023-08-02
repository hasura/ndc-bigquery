use super::sql_ast::*;

/// SQL AST builder helpers.
use std::collections::BTreeMap;

/// An empty `WITH` clause.
pub fn empty_with() -> With {
    With {
        recursive: false,
        common_table_expressions: vec![],
    }
}

/// An empty `WHERE` clause.
pub fn empty_where() -> Expression {
    Expression::Value(Value::Bool(true))
}

/// An empty `GROUP BY` clause.
pub fn empty_group_by() -> GroupBy {
    GroupBy {}
}

/// An empty `ORDER BY` clause.
pub fn empty_order_by() -> OrderBy {
    OrderBy { elements: vec![] }
}

/// Empty `LIMIT` and `OFFSET` clauses.
pub fn empty_limit() -> Limit {
    Limit {
        limit: None,
        offset: None,
    }
}

/// A `true` expression.
pub fn true_expr() -> Expression {
    Expression::Value(Value::Bool(true))
}

/// A `false` expression.
pub fn false_expr() -> Expression {
    Expression::Value(Value::Bool(false))
}

/// A table with the `public` schema.
impl TableName {
    pub fn public_table(tablename: String) -> TableName {
        TableName::DBTable {
            schema: "public".to_string(),
            table: tablename,
        }
    }
}

/// Build a simple select with a select list and the rest are empty.
pub fn simple_select(select_list: Vec<(ColumnAlias, Expression)>) -> Select {
    Select {
        with: empty_with(),
        select_list: SelectList(select_list),
        from: None,
        joins: vec![],
        where_: Where(empty_where()),
        group_by: empty_group_by(),
        order_by: empty_order_by(),
        limit: empty_limit(),
    }
}

/// Wrap a query in `SELECT coalesce(json_agg(row_to_json(<table_alias>)), '[]') AS <column_alias> FROM <query> as <table_alias>`.
///
/// - `row_to_json` takes a row and converts it to a json object.
/// - `json_agg` aggregates the json objects to a json array.
/// - `coalesce(<thing>, <otherwise>)` returns <thing> if it is not null, and <otherwise> if it is null.
pub fn select_table_as_json_array(
    select: Select,
    column_alias: ColumnAlias,
    table_alias: TableAlias,
) -> Select {
    let select_list = vec![(
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
                Expression::Value(Value::EmptyJsonArray),
            ],
        },
    )];

    let mut final_select = simple_select(select_list);
    final_select.from = Some(From::Select {
        select: Box::new(select),
        alias: table_alias,
    });
    final_select
}

/// Wrap a query in
///
/// > SELECT
/// >   json_build_object('rows', coalesce(json_agg(row_to_json(<table_alias>)), '[]')) AS <column_alias>
/// > FROM <query> as <table_alias>
///
/// - `row_to_json` takes a row and converts it to a json object.
/// - `json_agg` aggregates the json objects to a json array.
/// - `coalesce(<thing>, <otherwise>)` returns <thing> if it is not null, and <otherwise> if it is null.
/// - `json_build_object('rows', ...)` wraps that array in another object which looks like this
///   `{ "rows": [ {<row>}, ... ] }`, because that's what v3 engine expects for relationships.
pub fn select_table_as_json_array_in_rows_object(
    select: Select,
    column_alias: ColumnAlias,
    table_alias: TableAlias,
) -> Select {
    let select_list = vec![(
        column_alias,
        Expression::JsonBuildObject(BTreeMap::from([(
            "rows".to_string(),
            Box::new(Expression::FunctionCall {
                function: Function::Coalesce,
                args: vec![
                    Expression::FunctionCall {
                        function: Function::JsonAgg,
                        args: vec![Expression::RowToJson(TableName::AliasedTable(
                            table_alias.clone(),
                        ))],
                    },
                    Expression::Value(Value::EmptyJsonArray),
                ],
            }),
        )])),
    )];

    let mut final_select = simple_select(select_list);
    final_select.from = Some(From::Select {
        select: Box::new(select),
        alias: table_alias,
    });
    final_select
}

/// Wrap a query in `SELECT row_to_json(<table_alias>) AS <column_alias> FROM <query> as <table_alias>`.
///
/// - `row_to_json` takes a row and converts it to a json object.
/// - this is used in relationships where we expect a single row OR null
pub fn select_row_as_json(
    select: Select,
    column_alias: ColumnAlias,
    table_alias: TableAlias,
) -> Select {
    let select_list = vec![(
        column_alias,
        Expression::RowToJson(TableName::AliasedTable(table_alias.clone())),
    )];

    let mut final_select = simple_select(select_list);
    final_select.from = Some(From::Select {
        select: Box::new(select),
        alias: table_alias,
    });
    final_select
}
