pub mod common;

const CHINOOK_DEPLOYMENT_PATH: &str = "static/chinook-deployment.json";

#[tokio::test]
async fn get_schema() {
    let result = common::get_schema(CHINOOK_DEPLOYMENT_PATH).await;
    insta::assert_json_snapshot!(result);
}
