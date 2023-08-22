//! Test infrastructure for end-to-end tests.

use std::collections::HashMap;
use std::fs;

/// Given the name, we look for two files:
/// * `tests/goldenfiles/<name>.graphql` - the body of the GraphQL request to be sent.
/// * `tests/goldenfiles/<name>-variables.json` - if present, the object of variables to attach.
///
/// Using these, we make a request against the `v3-engine` running at `localhost:3000`
/// (specifically, we `POST` to the `/graphql` endpoint), and check the response against the
/// `goldenfiles`.
pub async fn run_graphql_against_v3_engine(graphql_body_name: &str) -> serde_json::Value {
    let body_path = format!("tests/goldenfiles/{}.graphql", graphql_body_name);
    let variables_path = format!("tests/goldenfiles/{}-variables.json", graphql_body_name);

    let body = match fs::read_to_string(body_path) {
        Ok(body) => serde_json::Value::String(body),
        Err(err) => {
            println!("Error: {}", err);
            panic!("error look up");
        }
    };

    let variables = match fs::read_to_string(variables_path) {
        Ok(variables) => serde_json::from_str(&variables).expect("Invalid JSON in variables file"),
        Err(_) => serde_json::Value::Null,
    };

    let map = HashMap::from([("query", body), ("variables", variables)]);

    reqwest::Client::new()
        .post("http://localhost:3000/graphql")
        .json(&map)
        .send()
        .await
        .expect("Failed to reach localhost:3000/graphql")
        .json()
        .await
        .expect("Failed to parse engine response as JSON")
}
