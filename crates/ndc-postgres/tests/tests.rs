mod common;
use std::env;

#[tokio::test]
async fn select_by_pk() {
    let result = common::run_query("select_by_pk").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_5() {
    let result = common::run_query("select_5").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_int_and_string() {
    let result = common::run_query("select_int_and_string").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_like() {
    let result = common::run_query("select_where_name_like").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_not_like() {
    let result = common::run_query("select_where_name_not_like").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_album_id_less_than() {
    let result = common::run_query("select_where_album_id_less_than").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_album_id_less_than_or_equal_to() {
    let result = common::run_query("select_where_album_id_less_than_or_equal_to").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_album_id_greater_than() {
    let result = common::run_query("select_where_album_id_greater_than").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_album_id_greater_than_or_equal_to() {
    let result = common::run_query("select_where_album_id_greater_than_or_equal_to").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_in() {
    let result = common::run_query("select_where_name_in").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_not_in() {
    let result = common::run_query("select_where_name_not_in").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_ilike() {
    let result = common::run_query("select_where_name_ilike").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_nilike() {
    let result = common::run_query("select_where_name_nilike").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_similar() {
    let result = common::run_query("select_where_name_similar").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_nsimilar() {
    let result = common::run_query("select_where_name_nsimilar").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_regex() {
    let result = common::run_query("select_where_name_regex").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_nregex() {
    let result = common::run_query("select_where_name_nregex").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_iregex() {
    let result = common::run_query("select_where_name_iregex").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_name_niregex() {
    let result = common::run_query("select_where_name_niregex").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_order_by_name() {
    let result = common::run_query("select_order_by_name").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_variable() {
    let result = common::run_query("select_where_variable").await;
    insta::assert_json_snapshot!(result);
}

#[tokio::test]
async fn select_where_variable_int() {
    let result = common::run_query("select_where_variable_int").await;
    insta::assert_json_snapshot!(result);
}

mod explain {
    use crate::common;
    use std::env;

    #[tokio::test]
    async fn select_by_pk() {
        let result = common::run_explain("select_by_pk").await;
        common::is_contained_in_lines(vec!["Aggregate", "Scan", "35"], result.lines);
        insta::assert_snapshot!(result.query);
    }

    #[tokio::test]
    async fn select_where_variable() {
        let result = common::run_explain("select_where_variable").await;
        common::is_contained_in_lines(vec!["Aggregate", "Seq Scan", "Filter"], result.lines);
        insta::assert_snapshot!(result.query);
    }

    #[tokio::test]
    async fn select_where_name_nilike() {
        let result = common::run_explain("select_where_name_nilike").await;
        let keywords = vec!["Aggregate", "Subquery Scan", "Limit", "Seq Scan", "Filter"];
        common::is_contained_in_lines(keywords, result.lines);
        insta::assert_snapshot!(result.query);
    }
}
