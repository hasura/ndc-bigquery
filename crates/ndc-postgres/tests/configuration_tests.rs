//! Tests that configuration generation has not changed.
//!
//! If you have changed it intentionally, run `just generate-chinook-configuration`.

pub mod common;

use std::fs;

use similar_asserts::assert_eq;

use ndc_postgres::configuration;

use crate::common::get_deployment_file;

const POSTGRESQL_CONNECTION_STRING: &str = "postgresql://postgres:password@localhost:64002";
const CHINOOK_DEPLOYMENT_PATH: &str = "static/chinook-deployment.json";

#[tokio::test]
async fn test_configure() {
    let args = configuration::DeploymentConfiguration {
        postgres_database_url: POSTGRESQL_CONNECTION_STRING.to_string(),
        ..configuration::DeploymentConfiguration::empty()
    };

    let expected_value: serde_json::Value = {
        let file =
            fs::File::open(get_deployment_file(CHINOOK_DEPLOYMENT_PATH)).expect("fs::File::open");
        serde_json::from_reader(file).expect("serde_json::from_reader")
    };

    let actual = configuration::configure(&args)
        .await
        .expect("configuration::configure");

    let actual_value = serde_json::to_value(actual).expect("serde_json::to_value");

    assert_eq!(expected_value, actual_value);
}
