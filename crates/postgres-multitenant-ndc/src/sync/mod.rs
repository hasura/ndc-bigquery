use crate::state::{update_deployments, ServerState};
use std::time::Duration;

// look in the deployments folder every 10 seconds and add any we find to the shared state
pub fn start_deployment_sync_thread(base_dir: String, state: ServerState) {
    tokio::spawn(async move {
        tracing::info!("Started deployments sync thread");
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let base_dir = base_dir.clone();
            let state = state.clone();
            tokio::spawn(async move {
                if let Err(err) = update_deployments(base_dir, state).await {
                    tracing::error!("Error while updating deployments: {}", err)
                }
            });
        }
    });
}
