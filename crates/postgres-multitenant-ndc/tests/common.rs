use axum::http::StatusCode;
use axum_test_helper::TestClient;
use postgres_multitenant_ndc::state::update_deployments;
use postgres_multitenant_ndc::{routes, state};
use std::fs;
use std::path::PathBuf;

use std::env;

/// Run a query against the server, get the result, and compare against the snapshot.
pub async fn test_query(testname: &str) -> String {
    // initialise empty server state
    let state = state::ServerState::default();

    // create a fresh router
    let router = routes::create_router(state.clone());

    // using the single static deployment from "./static" folder
    let deployment_name = "88011674-8513-4d6b-897a-4ab856e0bb8a".to_string();

    // work out where the deployment configs live
    let test_deployments_dir = get_deployments_dir();

    // check in folder for deployments, put them in state
    let _ = update_deployments(test_deployments_dir, state).await;

    let client = TestClient::new(router);
    let request = fs::read_to_string(format!("tests/goldenfiles/{}.json", testname)).unwrap();

    let url = format!("/deployment/{}/query", deployment_name);

    let res = client
        .post(&url)
        .body(request)
        .header("Content-Type", "application/json")
        .send()
        .await;

    assert_eq!(res.status(), StatusCode::OK);

    res.text().await
}

/// find the deployments folder via the crate root provided by `cargo test`.
fn get_deployments_dir() -> String {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("../../static/deployments");

    return d.display().to_string();
}
