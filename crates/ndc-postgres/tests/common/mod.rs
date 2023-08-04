//! Common functions used across test cases.

use std::fs;
use std::path::PathBuf;

use axum::http::StatusCode;
use axum_test_helper::TestClient;
use serde_derive::Deserialize;

use ndc_postgres::connector;

pub const POSTGRESQL_CONNECTION_STRING: &str = "postgresql://postgres:password@localhost:64002";

/// Run a query against the server, get the result, and compare against the snapshot.
pub async fn run_query(testname: &str) -> serde_json::Value {
    run_against_server("query", testname).await
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ExactExplainResponse {
    pub details: ExplainDetails,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ExplainDetails {
    #[serde(rename = "SQL Query")]
    pub query: String,
    #[serde(rename = "Execution Plan")]
    pub plan: String,
}

/// Run a query against the server, get the result, and compare against the snapshot.
pub async fn run_explain(testname: &str) -> ExactExplainResponse {
    let result = run_against_server("explain", testname).await;
    serde_json::from_value(result).unwrap()
}

/// Run an action against the server, get the result, and compare against the snapshot.
async fn run_against_server(action: &str, testname: &str) -> serde_json::Value {
    let _ = env_logger::builder().is_test(true).try_init();

    // work out where the deployment configs live
    let test_deployment_file = get_deployment_file();

    // initialise server state with the static configuration.
    let state = ndc_hub::default_main::init_server_state::<connector::Postgres>(
        test_deployment_file.display().to_string(),
    )
    .await;

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

/// Check if all keywords are contained in this vector of strings.
/// Used to check the output of EXPLAIN. We use this method instead of
/// snapshot testing because small details (like cost) can change from
/// run to run rendering the output unstable.
pub fn is_contained_in_lines(keywords: Vec<&str>, lines: String) {
    tracing::info!("expected keywords: {:?}\nlines: {}", keywords, lines,);
    assert!(keywords.iter().all(|&s| lines.contains(s)));
}

/// Find the project root via the crate root provided by `cargo test`,
/// and get our single static configuration file.
pub fn get_deployment_file() -> PathBuf {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("../../static/chinook-deployment.json");
    d
}
