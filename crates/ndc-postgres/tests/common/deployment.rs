use std::env;
use std::path::PathBuf;

/// find the deployments folder via the crate root provided by `cargo test`,
/// and get our single static configuration file.
pub fn get_deployment_file() -> String {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("../../static/chinook-deployment.json");

    return d.display().to_string();
}
