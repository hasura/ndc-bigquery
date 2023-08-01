use std::fs;

use axum::http::StatusCode;
use axum_test_helper::TestClient;

use ndc_postgres::connector;

use super::deployment::get_deployment_file;

/// Run a query against the server, get the result, and compare against the snapshot.
pub async fn run_query(testname: &str) -> serde_json::Value {
    run_against_server("query", testname).await
}

/// Run a query against the server, get the result, and compare against the snapshot.
pub async fn run_explain(testname: &str) -> ndc_client::models::ExplainResponse {
    let result = run_against_server("explain", testname).await;
    serde_json::from_value(result).unwrap()
}

/// Run an action against the server, get the result, and compare against the snapshot.
async fn run_against_server(action: &str, testname: &str) -> serde_json::Value {
    let _ = env_logger::builder().is_test(true).try_init();

    // work out where the deployment configs live
    let test_deployment_file = get_deployment_file();

    // initialise server state with the static configuration.
    let state =
        ndc_hub::default_main::init_server_state::<connector::Postgres>(test_deployment_file).await;

    // create a fresh router
    let router = ndc_hub::default_main::create_router(state);

    let client = TestClient::new(router);
    let request = match fs::read_to_string(format!("tests/goldenfiles/{}.json", testname)) {
        Ok(request) => request,
        Err(err) => {
            println!("Error: {}", err);
            panic!("error look up");
        }
    };

    let url = format!("/{}", action);

    let res = client
        .post(&url)
        .body(request)
        .header("Content-Type", "application/json")
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::OK);

    //serde_json::Value::String(res.text().await)
    res.json().await
}
