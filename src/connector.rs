/// The state of the connector.
use sqlx;
use std::env;

pub struct Connector {
    pub pg_pool: sqlx::PgPool,
}

impl Connector {
    pub async fn new() -> Result<Connector, sqlx::Error> {
        // set postgres URL via env var. maybe not the way, but for now, yolo.
        let postgresql_connect_string = env::var("POSTGRESQL_CONNECTION_STRING").unwrap_or_default();

        let pg_pool =
            sqlx::PgPool::connect(&postgresql_connect_string)
                .await?;
        Ok(Connector { pg_pool })
    }
}
