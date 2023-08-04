use super::ast::*;
use std::collections::BTreeMap;

pub enum SelectSet {
    Rows(Select),
    Aggregates(Select),
    RowsAndAggregates(Select, Select),
}

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
        select_list: SelectList::SelectList(select_list),
        from: None,
        joins: vec![],
        where_: Where(empty_where()),
        group_by: empty_group_by(),
        order_by: empty_order_by(),
        limit: empty_limit(),
    }
}

/// Build a simple select *
pub fn star_select(from: From) -> Select {
    Select {
        with: empty_with(),
        select_list: SelectList::SelectStar,
        from: Some(from),
        joins: vec![],
        where_: Where(empty_where()),
        group_by: empty_group_by(),
        order_by: empty_order_by(),
        limit: empty_limit(),
    }
}

/// given a set of rows and aggregate queries, combine them into
/// one Select
/// > SELECT row_to_json(<output_table_alias>) AS <output_column_alias>
/// > FROM (
/// >   SELECT *
/// >     FROM (
/// >       SELECT coalesce(json_agg(row_to_json(<row_column_alias>)), '[]') AS "rows"
/// >         FROM (<row_select>) AS <row_table_alias>
/// >       ) AS <row_column_alias>
/// >         CROSS JOIN (
/// >           SELECT coalesce(row_to_json(<aggregate_column_alias>), '[]') AS "aggregates"
/// >             FROM (<aggregate_select>) AS <aggregate_table_alias>
/// >           ) AS <aggregate_column_alias>
/// >        ) AS <output_column_alias>
///
/// The `row_select` and `aggregate_set` will not be included if they are not relevant
pub fn select_rowset(
    output_column_alias: ColumnAlias,
    output_table_alias: TableAlias,
    row_table_alias: TableAlias,
    row_column_alias: ColumnAlias,
    aggregate_table_alias: TableAlias,
    aggregate_column_alias: ColumnAlias,
    select_set: SelectSet,
) -> Select {
    let row = vec![(
        output_column_alias,
        (Expression::RowToJson(TableName::AliasedTable(output_table_alias.clone()))),
    )];

    let mut final_select = simple_select(row);

    let wrap_row =
        |row_sel| select_rows_as_json(row_sel, row_column_alias, row_table_alias.clone());

    let wrap_aggregate = |aggregate_sel| {
        select_row_as_json_with_default(
            aggregate_sel,
            aggregate_column_alias,
            aggregate_table_alias.clone(),
        )
    };

    match select_set {
        SelectSet::Rows(row_select) => {
            let select_star = star_select(From::Select {
                alias: row_table_alias.clone(),
                select: Box::new(wrap_row(row_select)),
            });
            final_select.from = Some(From::Select {
                alias: output_table_alias,
                select: Box::new(select_star),
            })
        }
        SelectSet::Aggregates(aggregate_select) => {
            let select_star = star_select(From::Select {
                alias: aggregate_table_alias.clone(),
                select: Box::new(wrap_aggregate(aggregate_select)),
            });
            final_select.from = Some(From::Select {
                alias: output_table_alias,
                select: Box::new(select_star),
            })
        }
        SelectSet::RowsAndAggregates(row_select, aggregate_select) => {
            let mut select_star = star_select(From::Select {
                alias: row_table_alias.clone(),
                select: Box::new(wrap_row(row_select)),
            });

            select_star.joins = vec![Join::CrossJoin(CrossJoin {
                select: Box::new(wrap_aggregate(aggregate_select)),
                alias: aggregate_table_alias.clone(),
            })];

            final_select.from = Some(From::Select {
                alias: output_table_alias,
                select: Box::new(select_star),
            })
        }
    }
    final_select
}

/// Wrap an query that returns multiple rows in
///
/// > SELECT
/// >   coalesce(json_agg(row_to_json(<table_alias>)), '[]')) AS <column_alias>
/// > FROM <query> as <table_alias>
///
/// - `row_to_json` takes a row and converts it to a json object.
/// - `json_agg` aggregates the json objects to a json array.
/// - `coalesce(<thing>, <otherwise>)` returns <thing> if it is not null, and <otherwise> if it is null.
///
pub fn select_rows_as_json(
    row_select: Select,
    column_alias: ColumnAlias,
    table_alias: TableAlias,
) -> Select {
    let expression = Expression::FunctionCall {
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
    };
    let mut select = simple_select(vec![(column_alias, expression)]);
    select.from = Some(From::Select {
        select: Box::new(row_select),
        alias: table_alias,
    });
    select
}

/// Wrap an query that returns a single row in
///
/// > SELECT
/// >   coalesce(row_to_json(<table_alias>), '{}'::json)) AS <column_alias>
/// > FROM <query> as <table_alias>
///
/// - `row_to_json` takes a row and converts it to a json object.
/// - `coalesce(<thing>, <otherwise>)` returns <thing> if it is not null, and <otherwise> if it is null.
///
pub fn select_row_as_json_with_default(
    select: Select,
    column_alias: ColumnAlias,
    table_alias: TableAlias,
) -> Select {
    let expression = Expression::FunctionCall {
        function: Function::Coalesce,
        args: vec![
            Expression::RowToJson(TableName::AliasedTable(table_alias.clone())),
            Expression::Value(Value::EmptyJsonArray),
        ],
    };
    let mut final_select = simple_select(vec![(column_alias, expression)]);
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
