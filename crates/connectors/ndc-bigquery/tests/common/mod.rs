//! Common functions used across test cases.

use std::collections::HashMap;

use ndc_postgres::connector;

pub const CHINOOK_DEPLOYMENT_PATH: &str = "static/";

pub const HASURA_BIGQUERY_SERVICE_KEY: &str = "{\"type\": \"service_account\",\"project_id\": \"hasura-development\",\"private_key_id\": \"222dd3f9e98b6743bb8d74d7a126fe89e6ac221d\",\"private_key\": \"-----BEGIN PRIVATE KEY-----\\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDZuxyxWk6bOxHr\\nht+MPZ7Q+F4D7AzTqCTZOmcldod+KHMlUCKwIOLQabAO8TEPhvcYyzBQ4gCwoN2i\\n7VoQbHmlTpQu1s43K25oIoEIicFTbHcL4MALiFnT44XYl+PxL+e//GibJYGjwqI+\\n3o2Go//o8BOfQEO/PPiQdub8/4VQjXGE0+xLeUjYURiJPu8ojrL2FdIqvzMaDWKq\\n2r6KuukIeWkqmt6fHnrGiseavg3g7pBPjqmRtX6ekY74XbkTQk1kmCKf9MLjZ1UI\\n+8QNp1C4pO4eDbp1Zkqz3uHhzccUvStkSCmppjKfD64Tp+6ExbUnMfq1UJ0GJBDM\\nVPeJF6+PAgMBAAECggEAFFSf88GKeH02CaeQ5S/1ze57HOOkOPlI443MBtgAA9w0\\nEEZgztBrTWmo+mQ0IA6KsSJ78vl/df63Y1jFYaY3X6OsO4lsPQONriSWptzyE9+b\\naB0G4azMMnhazaQ1MRa3jZo8jEwexFNOwg8W6P0UTsRoGKUwDkHbteWcYQBdCu3W\\nFa/CX3Tw0n/DdAVNi8Ai9K0d+Okmcv+ZRopeNuLENR28/VGSXj+Li1V7A0s+nX9E\\nyxuGrDY4WMxSXHkW2yjrDnPUs6dXLFk1HBQPaHrs3i6gGyNXfTNWUJ3nGQwZIqJI\\na1b4TMiGVapq33qCo/3Yi6jQ+I6KnpmWgQ7y5LXhoQKBgQDuA80oWCXQv7MERg91\\nFwammtXrMjoD234u3RGNtnU67yH87kvL+p18EiNlbmy+CWyoc1mOjLtTHvMBfMGh\\nfKt3BSuzrZZArA1GJF6J2Rew5dkJGzwPogLSnXMgrVwknAejKJw97wTJzzIZuuSc\\nb7P57+mFoSdR+eSb44WFcuMyoQKBgQDqLu9LWz+LcljDWDeMQ4kl8gkNZMe5//Qd\\nOpa6mN6T2nfRgxasaLo7WO8TqT4X28eBJKuru4BOeHVx0Y8GvWYyaW0uEEycdXVl\\n6man+YUhZezTjjB/nCeaz7E7LCcUao1JP2Y9xlnpO5jdyi2tYkCqu7vOxmnLArN/\\nl3zuXgrkLwKBgEzCzReF1ixMpt9p+PI6StrQdM01laBI2ZkjktWxUn1/Qebgs3FF\\nkiTBdMjxpABl6bUp/mgK2x8jjBuesJP0MRhhgoagJSUWV/GXKSYr7YgPmL9nGSex\\niFeEj+yp/F2SNKRaJImU3GZ5fB7wN2p8W/7vcNC3+IZnoWLlLdqsAroBAoGAdzZh\\nVoki9gfFq9uym1Kd9JUbipftHIBxcpeqt16un7GtIRiMaEP/2cpSGj4jf92/17wl\\nMA0JKekkUEaPeqzb43nLvJFLjrI0iyciDwx0eyX5w1A03CFP//0OicLWOgxr1AfU\\nMkpQ5uwRy4XqbsL/jGp5Fq/mlxPO8HrbfDSfcr0CgYEAxN/RMCYODz+p9xZ6tbiS\\nfHFrCgvPpYR9hEWhb/DyT4Q/OSzk0TItuSXGc3uicYeIycHIndyWej/a1HGg0IRK\\nqjGbqGvRJIrzhLvLog1oOGADFSE2IJrxV2m9lQG8IUow4QUFcoZaCXZAQEvWeo+D\\nq+4Pe2w4aMZeyqpt/mOSGzQ=\\n-----END PRIVATE KEY-----\\n\",\"client_email\": \"skm-bq-test@hasura-development.iam.gserviceaccount.com\",\"client_id\": \"116460406056940511807\",\"auth_uri\": \"https://accounts.google.com/o/oauth2/auth\",\"token_uri\": \"https://oauth2.googleapis.com/token\",\"auth_provider_x509_cert_url\": \"https://www.googleapis.com/oauth2/v1/certs\",\"client_x509_cert_url\": \"https://www.googleapis.com/robot/v1/metadata/x509/skm-bq-test%40hasura-development.iam.gserviceaccount.com\",\"universe_domain\": \"googleapis.com\"}";

pub const HASURA_BIGQUERY_PROJECT_ID: &str = "hasura-development";
pub const HASURA_BIGQUERY_DATASET_ID: &str = "chinook_sample";

/// Creates a router with a fresh state from the test deployment.
pub async fn create_router() -> axum::Router {
    create_router_from_deployment(CHINOOK_DEPLOYMENT_PATH).await
}

/// Creates a router with a fresh state from a deployment file path
pub async fn create_router_from_deployment(deployment_path: &str) -> axum::Router {
    let _ = env_logger::builder().is_test(true).try_init();

    let environment = HashMap::from([
        (
            ndc_bigquery_configuration::version1::DEFAULT_SERVICE_KEY_VARIABLE.into(),
            HASURA_BIGQUERY_SERVICE_KEY.to_string(),
        ),
        (
            ndc_bigquery_configuration::version1::DEFAULT_PROJECT_ID_VARIABLE.into(),
            HASURA_BIGQUERY_PROJECT_ID.to_string(),
        ),
        (
            ndc_bigquery_configuration::version1::DEFAULT_DATASET_ID_VARIABLE.into(),
            HASURA_BIGQUERY_DATASET_ID.to_string(),
        ),
    ]);

    let setup = connector::BigQuerySetup::new(environment);

    // work out where the deployment configs live
    let test_deployment_file =
        tests_common::deployment::helpers::get_path_from_project_root(deployment_path);

    // initialise server state with the static configuration.
    let state =
        ndc_sdk::default_main::init_server_state(setup, test_deployment_file.display().to_string())
            .await
            .unwrap();

    ndc_sdk::default_main::create_router(state, None)
}
