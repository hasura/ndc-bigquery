//! Helpers for building sql::ast types in certain shapes and patterns.

use super::ast::*;
use std::collections::BTreeMap;

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
/// Create column aliases using this function so we build everything in one place.
pub fn make_column_alias(name: String) -> ColumnAlias {
    ColumnAlias { name }
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
/// The `row_select` and `aggregate_set` will not be included if they are not relevant
pub fn select_rowset(
    output_column_alias: ColumnAlias,
    output_table_alias: TableAlias,
    row_table_alias: TableAlias,
    _row_column_alias: ColumnAlias,
    select_set: SelectSet,
) -> Select {
    let mut json_items = BTreeMap::new();

    json_items.insert(
        "rows".to_string(),
        Box::new(Expression::FunctionCall {
            function: Function::ArrayAgg,
            args: vec![Expression::TableReference(TableReference::AliasedTable(
                output_table_alias.clone(),
            ))],
        }),
    );

    let row = vec![(
        output_column_alias,
        (Expression::JsonBuildObject(json_items)),
    )];

    //  TableReference::AliasedTable(output_table_alias.clone()))),

    let mut final_select = simple_select(row);

    match select_set {
        SelectSet::Rows(row_select) => {
            let select_star = star_select(From::Select {
                alias: row_table_alias.clone(),
                select: Box::new(row_select),
            });
            final_select.from = Some(From::Select {
                alias: output_table_alias,
                select: Box::new(select_star),
            })
        }
        _ => todo!("no select rowset for aggregates"),
    }
    final_select
}
