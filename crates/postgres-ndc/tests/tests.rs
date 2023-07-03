use postgres_ndc::phases::translation::{select_to_sql, sql_ast, sql_string};

extern crate goldenfile;

use std::fs;

use insta;

use axum::http::StatusCode;
use axum_test_helper::TestClient;
use postgres_ndc::*;
use std::env;

#[tokio::test]
async fn select_by_pk() {
    let result = test_query("select_by_pk").await;
    insta::assert_snapshot!(result);
}

#[tokio::test]
async fn select_5() {
    let result = test_query("select_5").await;
    insta::assert_snapshot!(result);
}

async fn test_query(testdir: &str) -> String {
    match routes::router().await {
        Err(err) => {
            assert_eq!("cannot start server", err.to_string());
            err.to_string()
        }
        Ok(router) => {
            let client = TestClient::new(router);
            let request =
                fs::read_to_string(format!("tests/goldenfiles/{}.json", testdir)).unwrap();

            let res = client
                .post("/query")
                .body(request)
                .header("Content-Type", "application/json")
                .send()
                .await;

            assert_eq!(res.status(), StatusCode::OK);

            res.text().await
        }
    }
}

#[test]
fn it_converts_simple_select() {
    let select = sql_ast::simple_select(
        vec![(
            sql_ast::ColumnAlias {
                unique_index: 0,
                name: "x".to_string(),
            },
            sql_ast::Expression::ColumnName(sql_ast::ColumnName::TableColumn("x".to_string())),
        )],
        sql_ast::From::Table {
            name: sql_ast::TableName::DBTable("bamba".to_string()),
            alias: sql_ast::TableAlias {
                unique_index: 0,
                name: "bamba".to_string(),
            },
        },
    );
    assert_eq!(
        select_to_sql(&select),
        sql_string::SQL {
            //sql: "SELECT \"x\" AS \"hasu_col_0_x\" FROM \"bamba\" AS \"hasu_tbl_0_bamba\"".to_string(),
            sql: "SELECT \"x\" AS \"x\" FROM \"bamba\" AS \"bamba\"".to_string(),
            params: vec![],
            param_index: 0,
        }
    );
}
