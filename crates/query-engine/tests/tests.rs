use query_engine::phases::translation::{select_to_sql, sql_ast, sql_string};

#[test]
fn it_converts_simple_select() {
    let table_name = sql_ast::TableName::from_public("bamba".to_string());
    let table_alias = sql_ast::TableAlias {
        unique_index: 0,
        name: "bamba".to_string(),
    };
    let select = sql_ast::simple_select(
        vec![(
            sql_ast::ColumnAlias {
                unique_index: 0,
                name: "x".to_string(),
            },
            sql_ast::Expression::ColumnName(sql_ast::ColumnName::TableColumn {
                table: sql_ast::TableName::AliasedTable(table_alias.clone()),
                name: "x".to_string(),
            }),
        )],
        sql_ast::From::Table {
            name: table_name,
            alias: table_alias,
        },
    );
    assert_eq!(
        select_to_sql(&select),
        sql_string::SQL {
            sql: "SELECT \"bamba\".\"x\" AS \"x\" FROM \"public\".\"bamba\" AS \"bamba\""
                .to_string(),
            params: vec![],
            param_index: 0,
        }
    );
}
