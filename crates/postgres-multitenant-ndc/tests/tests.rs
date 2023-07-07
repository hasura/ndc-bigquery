mod common;
use std::env;

#[tokio::test]
async fn select_by_pk() {
    let result = common::test_query("select_by_pk").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_5() {
    let result = common::test_query("select_5").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_int_and_string() {
    let result = common::test_query("select_int_and_string").await;
    insta::assert_json_snapshot!(result);
}
