//! Database connection settings.

use crate::values::{DatasetId, ProjectId, Secret, ServiceKey, WorkloadIdentityAuth};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DEFAULT_SERVICE_KEY_VARIABLE: &str = "HASURA_BIGQUERY_SERVICE_KEY";
pub const DEFAULT_WORKLOAD_IDENTITY_AUTH_VARIABLE: &str = "HASURA_BIGQUERY_WORKLOAD_IDENTITY_AUTH";
pub const DEFAULT_PROJECT_ID_VARIABLE: &str = "HASURA_BIGQUERY_PROJECT_ID";
pub const DEFAULT_DATASET_ID_VARIABLE: &str = "HASURA_BIGQUERY_DATASET_ID";

/// Database connection settings.
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConnectionSettings {
    /// Connection string for a BigQuery-compatible database.
    pub service_key: Option<ServiceKey>,
    /// Connection string for a BigQuery-compatible database.
    pub workload_identity_auth: Option<WorkloadIdentityAuth>,
    /// Project ID for a BigQuery database.
    pub project_id: ProjectId,
    /// Dataset ID for a BigQuery database.
    pub dataset_id: DatasetId,
}

impl DatabaseConnectionSettings {
    pub fn empty() -> Self {
        Self {
            service_key: Some(ServiceKey(Secret::FromEnvironment {
                variable: DEFAULT_SERVICE_KEY_VARIABLE.into(),
            })),
            workload_identity_auth: Some(WorkloadIdentityAuth(Secret::FromEnvironment {
                variable: DEFAULT_WORKLOAD_IDENTITY_AUTH_VARIABLE.into(),
            })),
            project_id: ProjectId(Secret::FromEnvironment {
                variable: DEFAULT_PROJECT_ID_VARIABLE.into(),
            }),
            dataset_id: DatasetId(Secret::FromEnvironment {
                variable: DEFAULT_DATASET_ID_VARIABLE.into(),
            }),
        }
    }
}
