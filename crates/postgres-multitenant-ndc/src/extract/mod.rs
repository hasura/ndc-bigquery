use std::sync::Arc;

use axum::{
    async_trait,
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::state::create_pool;

use super::{
    error::ServerError,
    state::{DeploymentConfiguration, ServerState},
};

pub struct Configuration(pub Arc<DeploymentConfiguration>);
pub struct Pool(pub PgPool);

#[async_trait]
impl FromRequestParts<ServerState> for Configuration {
    type Rejection = ServerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ServerState,
    ) -> Result<Self, Self::Rejection> {
        let Path(deployment_id) = Path::<Uuid>::from_request_parts(parts, state)
            .await
            .map_err(|_err| ServerError::DeploymentIdMissingOrInvalid)?;
        if let Some(deployment_context) = state.deployments.read().await.get(&deployment_id) {
            Ok(Configuration(deployment_context.configuration.clone()))
        } else {
            Err(ServerError::DeploymentNotFound)
        }
    }
}

#[async_trait]
impl FromRequestParts<ServerState> for Pool {
    type Rejection = ServerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ServerState,
    ) -> Result<Self, Self::Rejection> {
        let Path(deployment_id) = Path::<Uuid>::from_request_parts(parts, state)
            .await
            .map_err(|_err| ServerError::DeploymentIdMissingOrInvalid)?;

        let pool = {
            state
                .deployments
                .read()
                .await
                .get(&deployment_id)
                .and_then(|ctx| ctx.pool.clone())
        };

        if let Some(pool) = pool {
            Ok(Pool(pool))
        } else if let Some(deployment_context) =
            state.deployments.write().await.get_mut(&deployment_id)
        {
            if let Some(pool) = &deployment_context.pool {
                // if another thread created a pool since we last checked, use that
                Ok(Pool(pool.clone()))
            } else {
                // else, create a new pool. Note we are holding on to the write lock while awaiting the pool creation.
                // in practice this should not be an issue, since other threads would need to create a pool anyways
                // this should guarantee that only one thread attempts to create a pool at a time.
                let pool = create_pool(&deployment_context.configuration).await?;
                deployment_context.pool = Some(pool.clone());
                Ok(Pool(pool))
            }
        } else {
            Err(ServerError::DeploymentNotFound)
        }
    }
}
