//! Helpers for building sql::ast types in certain shapes and patterns.

use super::ast::*;

/// Used as input to helpers to construct SELECTs which return 'rows' and/or 'aggregates' results.
pub enum SelectSet {
    Rows(Select),
    Aggregates(Select),
    RowsAndAggregates(Select, Select),
}

// Empty clauses //

/// An empty `WITH` clause.
pub fn empty_with() -> With {
    With {
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

// Aliasing //

/// Generate a column expression refering to a specific table.
pub fn make_column(
    table: TableReference,
    name: ColumnName,
    alias: ColumnAlias,
) -> (ColumnAlias, Expression) {
    (
        alias,
        Expression::ColumnReference(ColumnReference::TableColumn { table, name }),
    )
}

/// Create a table alias for boolean expressions.
/// Provide state for fresh names and a source table name (to point at the table
/// being filtered), and get an alias.
pub fn make_boolean_expression_table_alias(
    next_fresh_name: &mut u32,
    source_table_name: &String,
) -> TableAlias {
    *next_fresh_name += 1;
    make_table_alias(format!(
        "%BOOLEXP_{}_FOR_{}",
        next_fresh_name, source_table_name
    ))
}

/// Create column aliases using this function so we build everything in one place.
pub fn make_column_alias(name: String) -> ColumnAlias {
    ColumnAlias { name }
}
/// Create table aliases using this function so they get a unique index.
pub fn make_table_alias(name: String) -> TableAlias {
    TableAlias { name }
}

/// Create a table alias for left outer join lateral part.
/// Provide an index and a source table name so we avoid name clashes,
/// and get an alias.
pub fn make_relationship_table_alias(index: usize, name: &String) -> TableAlias {
    make_table_alias(format!("%RELATIONSHIP_{}_FOR_{}", index, name))
}

/// Create a table alias for order by target part.
/// Provide an index and a source table name (to disambiguate the table being queried),
/// and get an alias.
pub fn make_order_path_part_table_alias(index: usize, table_name: &String) -> TableAlias {
    make_table_alias(format!("%ORDER_PATH_PART_{}_{}", index, table_name))
}

/// Create a table alias for order by column.
/// Provide an index and a source table name (to point at the table being ordered),
/// and get an alias.
pub fn make_order_by_table_alias(index: usize, source_table_name: &String) -> TableAlias {
    make_table_alias(format!("%ORDER_{}_FOR_{}", index, source_table_name))
}

/// Create a table alias for count aggregate order by column.
/// Provide an index and a source table name /// (to point at the table being ordered),
/// and get an alias.
pub fn make_order_by_count_table_alias(index: usize, source_table_name: &String) -> TableAlias {
    make_table_alias(format!("%ORDER_{}_COUNT_FOR_{}", index, source_table_name))
}

pub fn make_native_query_table_alias(index: usize, name: &String) -> TableAlias {
    make_table_alias(format!("%NATIVE_QUERY_{}_FOR_{}", index, name))
}

// SELECTs //

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
///
/// ```sql
/// SELECT row_to_json(<output_table_alias>) AS <output_column_alias>
/// FROM (
///   SELECT *
///     FROM (
///       SELECT coalesce(json_agg(row_to_json(<row_column_alias>)), '[]') AS "rows"
///         FROM (<row_select>) AS <row_table_alias>
///       ) AS <row_column_alias>
///         CROSS JOIN (
///           SELECT coalesce(row_to_json(<aggregate_column_alias>), '[]') AS "aggregates"
///             FROM (<aggregate_select>) AS <aggregate_table_alias>
///           ) AS <aggregate_column_alias>
///        ) AS <output_column_alias>
/// ```
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
        (Expression::RowToJson(TableReference::AliasedTable(output_table_alias.clone()))),
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
/// ```sql
/// SELECT
///   coalesce(json_agg(row_to_json(<table_alias>)), '[]')) AS <column_alias>
/// FROM <query> as <table_alias>
/// ```
///
/// - `row_to_json` takes a row and converts it to a json object.
/// - `json_agg` aggregates the json objects to a json array.
/// - `coalesce(<thing>, <otherwise>)` returns `<thing>` if it is not null, and `<otherwise>` if it is null.
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
                args: vec![Expression::RowToJson(TableReference::AliasedTable(
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
/// ```sql
/// SELECT
///   coalesce(row_to_json(<table_alias>), '{}'::json)) AS <column_alias>
/// FROM <query> as <table_alias>
/// ```
///
/// - `row_to_json` takes a row and converts it to a json object.
/// - `coalesce(<thing>, <otherwise>)` returns `<thing>` if it is not null, and `<otherwise>` if it is null.
///
pub fn select_row_as_json_with_default(
    select: Select,
    column_alias: ColumnAlias,
    table_alias: TableAlias,
) -> Select {
    let expression = Expression::FunctionCall {
        function: Function::Coalesce,
        args: vec![
            Expression::RowToJson(TableReference::AliasedTable(table_alias.clone())),
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
