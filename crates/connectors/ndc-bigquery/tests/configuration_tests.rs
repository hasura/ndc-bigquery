//! Tests that configuration generation has not changed.
//!
//! If you have changed it intentionally, run `just generate-chinook-configuration`.

pub mod common;

use std::{collections::HashMap, fs};

use similar_asserts::assert_eq;

use ndc_bigquery_configuration::{
    values::Secret,
    version1::{self, DEFAULT_SERVICE_KEY_VARIABLE},
    ConnectionUri,
};

use tests_common::deployment::helpers::get_path_from_project_root;

const POSTGRESQL_CONNECTION_STRING: &str = "postgresql://postgres:password@localhost:64002";
const CHINOOK_DEPLOYMENT_PATH: &str = "static/chinook-deployment.json";
const _CONFIGURATION_QUERY: &str = include_str!("../src/config2.sql");

#[tokio::test]
#[ignore]
async fn test_configure() {
    let expected_value: serde_json::Value = {
        let file = fs::File::open(get_path_from_project_root(CHINOOK_DEPLOYMENT_PATH))
            .expect("fs::File::open");
        let result: serde_json::Value =
            serde_json::from_reader(file).expect("serde_json::from_reader");

        result
    };

    let mut args: version1::ParsedConfiguration = serde_json::from_value(expected_value.clone())
        .expect("Unable to deserialize as RawConfiguration");

    let environment = HashMap::from([(
        version1::DEFAULT_SERVICE_KEY_VARIABLE.into(),
        POSTGRESQL_CONNECTION_STRING.into(),
    )]);

    args.service_key = ConnectionUri(Secret::Plain(DEFAULT_SERVICE_KEY_VARIABLE.to_string()));

    let actual = version1::configure(&args, environment)
        .await
        .expect("configuration::configure");

    let actual_value = serde_json::to_value(actual).expect("serde_json::to_value");

    assert_eq!(expected_value, actual_value);
}

#[tokio::test]
#[ignore]
async fn get_rawconfiguration_schema() {
    let schema = schemars::schema_for!(version1::ParsedConfiguration);
    insta::assert_json_snapshot!(schema);
}

#[tokio::test]
async fn get_configuration_schema() {
    let schema = schemars::schema_for!(version1::Configuration);
    insta::assert_json_snapshot!(schema);
}
