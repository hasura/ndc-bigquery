use std::{
    collections::HashMap,
    error::Error,
    fmt::{Display, Formatter},
};

use gdc_client::models::{self, QueryRequest};
use sqlparser::ast::{
    BinaryOperator, Expr, Function, FunctionArg, FunctionArgExpr, Ident, ObjectName, Offset,
    OffsetRows, Query, Select, SelectItem, SetExpr, Statement, TableAlias, TableFactor,
    TableWithJoins, UnaryOperator, Value,
};

use crate::state::DeploymentConfiguration;

#[derive(Debug)]
pub enum QueryBuilderError {
    /// The query had no fields or aggregates
    NoRowsOrAggregates,
}

impl Display for QueryBuilderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryBuilderError::NoRowsOrAggregates => {
                write!(f, "Query must have at least either fields or aggregates")
            }
        }
    }
}
impl Error for QueryBuilderError {}

pub enum BoundParam {
    Number(u32),
    Value(serde_json::Value),
}

pub struct QueryBuilder<'a> {
    configuration: &'a DeploymentConfiguration,
    request: &'a QueryRequest,
    bound_parameters: Vec<BoundParam>,
}

pub type QueryBuilderResult = Result<(Statement, Vec<BoundParam>), QueryBuilderError>;

impl<'a> QueryBuilder<'a> {
    pub fn build_sql(
        request: &'a QueryRequest,
        configuration: &'a DeploymentConfiguration,
    ) -> QueryBuilderResult {
        let mut builder = Self {
            request,
            configuration,
            bound_parameters: vec![],
        };
        let statement = Statement::Query(builder.root_query()?);
        Ok((statement, builder.bound_parameters))
    }
    // relying on postgres not caring if the order bound parameters show up in the generated sql
    // does not match the order they are provided, as long as the index is correct.
    fn bind_parameter(&mut self, param: BoundParam) -> Expr {
        self.bound_parameters.push(param);
        let placeholder_string = format!("${}", self.bound_parameters.len());
        Expr::Value(Value::Placeholder(placeholder_string))
    }
    fn root_query(&mut self) -> Result<Box<Query>, QueryBuilderError> {
        self.node_subquery(&self.request.table, &self.request.query)
    }
    fn node_subquery(
        &mut self,
        table_name: &'a str,
        query: &'a models::Query,
    ) -> Result<Box<Query>, QueryBuilderError> {
        let wrapper_projection = match (&query.fields, &query.aggregates) {
            (None, None) => return Err(QueryBuilderError::NoRowsOrAggregates),
            (None, Some(aggregates)) => {
                let aggregates_subquery =
                    self.aggregates_json_subquery(aggregates, table_name, query)?;
                vec![SelectItem::ExprWithAlias {
                    expr: Expr::Subquery(aggregates_subquery),
                    alias: sql_quoted_identifier("aggregates"),
                }]
            }
            (Some(fields), None) => {
                let rows_subquery = self.rows_json_subquery(fields, table_name, query)?;
                vec![SelectItem::ExprWithAlias {
                    expr: Expr::Subquery(rows_subquery),
                    alias: sql_quoted_identifier("rows"),
                }]
            }
            (Some(fields), Some(aggregates)) => {
                let rows_subquery = self.rows_json_subquery(fields, table_name, query)?;
                let aggregates_subquery =
                    self.aggregates_json_subquery(aggregates, table_name, query)?;

                vec![
                    SelectItem::ExprWithAlias {
                        expr: Expr::Subquery(rows_subquery),
                        alias: sql_quoted_identifier("rows"),
                    },
                    SelectItem::ExprWithAlias {
                        expr: Expr::Subquery(aggregates_subquery),
                        alias: sql_quoted_identifier("aggregates"),
                    },
                ]
            }
        };

        let wrapper_subquery = simple_sql_subquery(wrapper_projection, vec![]);

        // COALESCE(json_agg(to_json("node")), json_build_array()) AS "node"
        let node_projection = vec![SelectItem::ExprWithAlias {
            expr: sql_function(
                "COALESCE",
                vec![
                    sql_function(
                        "json_agg",
                        vec![sql_function(
                            "to_json",
                            vec![Expr::Identifier(sql_quoted_identifier("_wrapper"))],
                        )],
                    ),
                    sql_function("json_build_array", vec![]),
                ],
            ),
            alias: sql_quoted_identifier("_node"),
        }];

        let node_from = vec![TableWithJoins {
            relation: TableFactor::Derived {
                lateral: false,
                subquery: wrapper_subquery,
                alias: Some(TableAlias {
                    columns: vec![],
                    name: sql_quoted_identifier("_wrapper"),
                }),
            },
            joins: vec![],
        }];

        let node_subquery = simple_sql_subquery(node_projection, node_from);

        Ok(node_subquery)
    }
    fn rows_json_subquery(
        &mut self,
        fields: &'a HashMap<String, models::Field>,
        table_name: &'a str,
        query: &'a models::Query,
    ) -> Result<Box<Query>, QueryBuilderError> {
        let row_subquery = self.rows_subquery(fields, table_name, query)?;
        let rows_projection = vec![SelectItem::ExprWithAlias {
            expr: sql_function(
                "COALESCE",
                vec![
                    sql_function(
                        "json_agg",
                        vec![if fields.is_empty() {
                            sql_function("json_build_object", vec![])
                        } else {
                            sql_function(
                                "to_json",
                                vec![Expr::Identifier(sql_quoted_identifier("_rows"))],
                            )
                        }],
                    ),
                    sql_function("json_build_array", vec![]),
                ],
            ),
            alias: sql_quoted_identifier("rows"),
        }];
        let rows_from = vec![TableWithJoins {
            joins: vec![],
            relation: TableFactor::Derived {
                lateral: false,
                subquery: row_subquery,
                alias: Some(TableAlias {
                    name: sql_quoted_identifier("_rows"),
                    columns: vec![],
                }),
            },
        }];
        Ok(simple_sql_subquery(rows_projection, rows_from))
    }
    fn rows_subquery(
        &mut self,
        fields: &'a HashMap<String, models::Field>,
        table_name: &'a str,
        query: &'a models::Query,
    ) -> Result<Box<Query>, QueryBuilderError> {
        let rows_projection = if fields.is_empty() {
            vec![SelectItem::UnnamedExpr(Expr::Value(Value::Null))]
        } else {
            fields
                .iter()
                .map(|(alias, field)| SelectItem::ExprWithAlias {
                    expr: sql_function(
                        "json_build_object",
                        vec![
                            Expr::Value(Value::SingleQuotedString("value".to_owned())),
                            match field {
                                models::Field::Column { column, .. } => {
                                    let table_info = self
                                        .configuration
                                        .tables
                                        .get(table_name)
                                        .expect("tables should be in configuration");
                                    let column_info = table_info
                                        .columns
                                        .get(column)
                                        .expect("column should be in table");
                                    Expr::CompoundIdentifier(vec![
                                        sql_quoted_identifier("_origin"),
                                        sql_quoted_identifier(&column_info.name),
                                    ])
                                }
                                models::Field::Relationship { .. } => todo!(),
                            },
                        ],
                    ),
                    alias: sql_quoted_identifier(alias),
                })
                .collect()
        };

        let table_info = self
            .configuration
            .tables
            .get(table_name)
            .expect("Table should be in configuration");

        let rows_from = vec![TableWithJoins {
            joins: vec![],
            relation: TableFactor::Table {
                // note: assuming the table name is not aliased in any way, will need to change this
                name: ObjectName(vec![
                    sql_quoted_identifier(&table_info.schema_name),
                    sql_quoted_identifier(&table_info.table_name),
                ]),
                alias: Some(TableAlias {
                    name: sql_quoted_identifier("_origin"),
                    columns: vec![],
                }),
                args: None,
                with_hints: vec![],
            },
        }];

        let limit = query
            .limit
            .map(|limit| self.bind_parameter(BoundParam::Number(limit)));
        let offset = query.offset.map(|offset| Offset {
            value: self.bind_parameter(BoundParam::Number(offset)),
            rows: OffsetRows::None,
        });

        let rows_selection = query
            .predicate
            .as_ref()
            .map(|expr| self.predicate_expr(expr));
        Ok(Box::new(Query {
            with: None,
            body: Box::new(SetExpr::Select(Box::new(Select {
                distinct: None,
                top: None,
                projection: rows_projection,
                into: None,
                from: rows_from,
                lateral_views: vec![],
                selection: rows_selection,
                group_by: vec![],
                cluster_by: vec![],
                distribute_by: vec![],
                sort_by: vec![],
                having: None,
                qualify: None,
                named_window: vec![],
            }))),
            limit,
            offset,
            order_by: vec![],
            locks: vec![],
            fetch: None,
        }))
    }
    fn aggregates_json_subquery(
        &mut self,
        _aggregates: &'a HashMap<String, models::Aggregate>,
        _table_name: &'a str,
        _query: &'a models::Query,
    ) -> Result<Box<Query>, QueryBuilderError> {
        todo!("aggregates not implemented")
    }
    fn _aggregates_subquery(
        &mut self,
        _aggregates: &'a HashMap<String, models::Aggregate>,
        _table_name: &'a str,
        _query: &'a models::Query,
    ) -> Result<Box<Query>, QueryBuilderError> {
        todo!("aggregates not implemented")
    }
    fn predicate_expr(&mut self, expr: &'a models::Expression) -> Expr {
        match expr {
            models::Expression::And { expressions } => expressions
                .iter()
                .map(|e| self.predicate_expr(e))
                .reduce(sql_and_expr)
                .map(|e| match e {
                    Expr::BinaryOp {
                        op: BinaryOperator::And,
                        ..
                    } => Expr::Nested(Box::new(e)),
                    _ => e,
                })
                .unwrap_or_else(|| Expr::Value(Value::Boolean(true))),
            models::Expression::Or { expressions } => expressions
                .iter()
                .map(|e| self.predicate_expr(e))
                .reduce(sql_or_expr)
                .map(|e| match e {
                    Expr::BinaryOp {
                        op: BinaryOperator::Or,
                        ..
                    } => Expr::Nested(Box::new(e)),
                    _ => e,
                })
                .unwrap_or_else(|| Expr::Value(Value::Boolean(false))),
            models::Expression::Not { expression } => Expr::UnaryOp {
                op: UnaryOperator::Not,
                expr: Box::new(self.predicate_expr(expression)),
            },
            models::Expression::UnaryComparisonOperator { .. } => todo!(),
            models::Expression::BinaryComparisonOperator {
                column,
                operator,
                value,
            } => {
                // this is silly. Todo: remove redundant boxes from input types.
                let operator = &**operator;
                let column = &**column;
                let value = &**value;

                let operator = match operator {
                    models::BinaryComparisonOperator::Equal => BinaryOperator::Eq,
                    models::BinaryComparisonOperator::Other { .. } => {
                        todo!("Only the equals operator is supported")
                    }
                };

                let left = match column {
                    models::ComparisonTarget::RootTableColumn { name } => {
                        Expr::CompoundIdentifier(vec![
                            sql_quoted_identifier("_origin"),
                            sql_quoted_identifier(name),
                        ])
                    }
                    models::ComparisonTarget::Column { name, path } => {
                        if !path.is_empty() {
                            todo!("comparison against other tables not supported")
                        }
                        Expr::CompoundIdentifier(vec![
                            sql_quoted_identifier("_origin"),
                            sql_quoted_identifier(name),
                        ])
                    }
                };

                let right = match value {
                    models::ComparisonValue::Column { .. } => {
                        todo!("Column comparison not supported")
                    }
                    models::ComparisonValue::Scalar { value } => {
                        self.bind_parameter(BoundParam::Value(value.clone()))
                    }
                    models::ComparisonValue::Variable { .. } => {
                        todo!("Not sure what variable comparison is")
                    }
                };

                Expr::BinaryOp {
                    left: Box::new(left),
                    op: operator,
                    right: Box::new(right),
                }
            }

            models::Expression::BinaryArrayComparisonOperator { .. } => todo!(),
            models::Expression::Exists { .. } => todo!(),
        }
    }
}

fn simple_sql_subquery(projection: Vec<SelectItem>, from: Vec<TableWithJoins>) -> Box<Query> {
    Box::new(Query {
        with: None,
        body: Box::new(SetExpr::Select(Box::new(Select {
            distinct: None,
            top: None,
            projection,
            into: None,
            from,
            lateral_views: vec![],
            selection: None,
            group_by: vec![],
            cluster_by: vec![],
            distribute_by: vec![],
            sort_by: vec![],
            having: None,
            qualify: None,
            named_window: vec![],
        }))),
        limit: None,
        offset: None,
        order_by: vec![],
        locks: vec![],
        fetch: None,
    })
}

fn sql_function(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Function(Function {
        name: ObjectName(vec![Ident::new(name)]),
        args: args
            .into_iter()
            .map(|arg| FunctionArg::Unnamed(FunctionArgExpr::Expr(arg)))
            .collect(),
        over: None,
        distinct: false,
        special: false,
        order_by: vec![],
    })
}

fn sql_quoted_identifier<S: Into<String>>(value: S) -> Ident {
    Ident::with_quote('"', value)
}

pub fn sql_and_expr(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::And,
        right: Box::new(right),
    }
}
pub fn sql_or_expr(left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        left: Box::new(left),
        op: BinaryOperator::Or,
        right: Box::new(right),
    }
}
