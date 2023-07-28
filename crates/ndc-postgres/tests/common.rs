use axum::http::StatusCode;
use axum_test_helper::TestClient;
use ndc_client::models::ExplainResponse;
use ndc_hub;
use ndc_postgres::connector;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Run a query against the server, get the result, and compare against the snapshot.
pub async fn run_query(testname: &str) -> serde_json::Value {
    run_against_server("query", testname).await
}

/// Run a query against the server, get the result, and compare against the snapshot.
pub async fn run_explain(testname: &str) -> ExplainResponse {
    let result = run_against_server("explain", testname).await;
    serde_json::from_value(result).unwrap()
}

/// Check if all keywords are contained in this vector of strings.
/// Used to check the output of EXPLAIN. We use this method instead of
/// snapshot testing because small details (like cost) can change from
/// run to run rendering the output unstable.
pub fn is_contained_in_lines(keywords: Vec<&str>, lines: Vec<String>) {
    let connected_lines = lines.join("\n");
    tracing::info!(
        "expected keywords: {:?}\nlines: {}",
        keywords,
        connected_lines,
    );
    assert!(keywords.iter().all(|&s| connected_lines.contains(s)));
}

/// Run an action against the server, get the result, and compare against the snapshot.
async fn run_against_server(action: &str, testname: &str) -> serde_json::Value {
    let _ = env_logger::builder().is_test(true).try_init();

    // work out where the deployment configs live
    let test_deployment_file = get_deployment_file();

    // initialise server state with the static configuration.
    let state = ndc_hub::default_main::init_server_state::<connector::Postgres>(format!(
        "{}",
        test_deployment_file
    ))
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

/// find the deployments folder via the crate root provided by `cargo test`,
/// and get our single static configuration file.
fn get_deployment_file() -> String {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("../../static/chinook-deployment.json");

    return d.display().to_string();
}
