mod common;

use common::assertions::is_contained_in_lines;
use common::requests::run_explain;

#[tokio::test]
async fn select_by_pk() {
    let result = run_explain("select_by_pk").await;
    is_contained_in_lines(vec!["Aggregate", "Scan", "35"], result.lines);
    insta::assert_snapshot!(result.query);
}

#[tokio::test]
async fn select_where_variable() {
    let result = run_explain("select_where_variable").await;
    is_contained_in_lines(vec!["Aggregate", "Seq Scan", "Filter"], result.lines);
    insta::assert_snapshot!(result.query);
}

#[tokio::test]
async fn select_where_name_nilike() {
    let result = run_explain("select_where_name_nilike").await;
    let keywords = vec!["Aggregate", "Subquery Scan", "Limit", "Seq Scan", "Filter"];
    is_contained_in_lines(keywords, result.lines);
    insta::assert_snapshot!(result.query);
}
