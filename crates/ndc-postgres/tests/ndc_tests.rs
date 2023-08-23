//! Runs the tests provided by the ndc-spec.

pub mod common;

use std::net;
const CHINOOK_DEPLOYMENT_PATH: &str = "static/chinook-deployment.json";

#[tokio::test]
async fn test_connector() -> Result<(), ndc_client::apis::Error> {
    let router = common::create_router(CHINOOK_DEPLOYMENT_PATH).await;
    let server = hyper::Server::bind(&net::SocketAddr::new(
        net::IpAddr::V4(net::Ipv4Addr::LOCALHOST),
        0,
    ))
    .serve(router.into_make_service());

    let base_path = format!("http://{}", server.local_addr());
    eprintln!("Starting the server on {}", base_path);

    tokio::task::spawn(async {
        if let Err(err) = server.await {
            eprintln!("Server error: {}", err);
        }
    });

    let configuration = ndc_client::apis::configuration::Configuration {
        base_path,
        user_agent: None,
        client: reqwest::Client::new(),
        basic_auth: None,
        oauth_access_token: None,
        bearer_access_token: None,
        api_key: None,
    };

    ndc_test::test_connector(&configuration).await
}
