use gdc_client::models::SchemaResponse;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::error::Error;
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::{fs, sync::RwLock};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct DeploymentConfiguration {
    pub tables: HashMap<String, TableInfo>,
    pub schema: SchemaResponse,
    pub postgres_database_url: String,
}

#[derive(Debug, Deserialize)]
pub struct TableInfo {
    pub schema_name: String,
    pub table_name: String,
    pub columns: HashMap<String, ColumnInfo>,
}

#[derive(Debug, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DeploymentContext {
    pub configuration: Arc<DeploymentConfiguration>,
    pub pool: Option<PgPool>, // connection pool goes here
}

#[derive(Debug, Default, Clone)]
pub struct ServerState {
    pub deployments: Arc<RwLock<HashMap<Uuid, DeploymentContext>>>,
}

pub async fn update_deployments(
    base_dir: String,
    state: ServerState,
) -> Result<(), Box<dyn Error>> {
    let path = Path::new(&base_dir);
    let deployment_ids = get_list_of_deployments(path).await?;

    let (new_deployments, must_remove_deployments) = {
        let deployments = state.deployments.read().await;

        let mut new_deployments = vec![];

        for deployment_id in &deployment_ids {
            if !deployments.contains_key(deployment_id) {
                let filename = format!("{base_dir}/{deployment_id}.json");
                let path = Path::new(&filename);
                let configuration = read_deployment_configuration(path).await;

                match configuration {
                    Ok(configuration) => {
                        let deployment_context = create_deployment_context(configuration).await;

                        new_deployments.push((deployment_id.to_owned(), deployment_context));
                    }
                    Err(err) => {
                        // todo: add proper logging.
                        // We purposefully don't fail the entire function,
                        // so that issues reading one deployment won't affect others.
                        log::error!("There was an issue reading configuration for deployment {deployment_id}: {}", err)
                    }
                }
            }
        }

        let must_remove_deployments = deployments
            .keys()
            .any(|deployment_id| !deployment_ids.contains(deployment_id));

        (new_deployments, must_remove_deployments)
    };

    // only try to aquire a write lock if needed.
    if !new_deployments.is_empty() || must_remove_deployments {
        let mut deployments = state.deployments.write().await;

        deployments.retain(|deployment_id, _| deployment_ids.contains(deployment_id));

        deployments.extend(new_deployments.into_iter());
    }

    Ok(())
}

async fn get_list_of_deployments(path: &Path) -> Result<Vec<Uuid>, std::io::Error> {
    let mut dir = fs::read_dir(path).await?;
    let mut deployments = vec![];

    while let Some(entry) = dir.next_entry().await? {
        let path = entry.path();
        if path.is_file() && path.extension().unwrap_or_default() == "json" {
            if let Some(file_stem) = path.file_stem() {
                if let Some(file_stem) = file_stem.to_str() {
                    if let Ok(uuid) = Uuid::try_parse(file_stem) {
                        deployments.push(uuid)
                    }
                }
            }
        }
    }

    Ok(deployments)
}

async fn read_deployment_configuration(
    path: &Path,
) -> Result<DeploymentConfiguration, std::io::Error> {
    let file = fs::read_to_string(path).await?;
    let configuration: DeploymentConfiguration = serde_json::from_str(&file)?;
    Ok(configuration)
}

async fn create_deployment_context(configuration: DeploymentConfiguration) -> DeploymentContext {
    // silently throw away any error.
    // issues connecting to the database should not prevent context initialization
    let pool = create_pool(&configuration)
        .await
        .map_err(|err| {
            // TODO: add proper logging in case of error.
            // be mindful about logging credentials
            log::error!("There was an issue creating the connection pool: {}", err);
            err
        })
        .ok();
    DeploymentContext {
        configuration: Arc::new(configuration),
        pool,
    }
}

pub async fn create_pool(configuration: &DeploymentConfiguration) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&configuration.postgres_database_url)
        .await
}
