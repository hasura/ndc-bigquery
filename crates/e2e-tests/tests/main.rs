mod common;

#[tokio::test]
async fn smoke() {
    let result = common::run_graphql_against_v3_engine("smoke").await;
    insta::assert_json_snapshot!(result);
}
