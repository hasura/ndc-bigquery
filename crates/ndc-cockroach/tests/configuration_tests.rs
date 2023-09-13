//! Tests that configuration generation has not changed.
//!
//! If you have changed it intentionally, run `just generate-chinook-configuration`.

pub mod common;

use std::fs;

use similar_asserts::assert_eq;

use tests_common::deployment::get_deployment_file;

const CONFIGURATION_QUERY: &str = include_str!("../src/configuration.sql");

#[tokio::test]
async fn test_configure() {
    let args = ndc_postgres::configuration::RawConfiguration {
        postgres_database_url: ndc_postgres::configuration::PostgresDatabaseUrls::SingleRegion(
            common::POSTGRESQL_CONNECTION_STRING.to_string(),
        ),
        ..ndc_postgres::configuration::RawConfiguration::empty()
    };

    let expected_value: serde_json::Value = {
        let file = fs::File::open(get_deployment_file(common::CHINOOK_DEPLOYMENT_PATH))
            .expect("fs::File::open");
        let mut result: serde_json::Value =
            serde_json::from_reader(file).expect("serde_json::from_reader");

        // native queries cannot be configured from the database alone,
        // so we ignore the native queries in the configuration file
        // for the purpose of comparing the checked in file with the comparison.
        result["metadata"]["native_queries"]
            .as_object_mut()
            .unwrap()
            .clear();
        result
    };

    let actual = ndc_postgres::configuration::configure(&args, CONFIGURATION_QUERY)
        .await
        .expect("configuration::configure");

    let actual_value = serde_json::to_value(actual).expect("serde_json::to_value");

    assert_eq!(expected_value, actual_value);
}
