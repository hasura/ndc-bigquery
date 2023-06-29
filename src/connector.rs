/// The state of the connector.
use sqlx;

pub struct Connector {
    pub pg_pool: sqlx::PgPool,
}

impl Connector {
    pub async fn new() -> Result<Connector, sqlx::Error> {
        let pg_pool =
            sqlx::PgPool::connect("postgresql://postgres:postgres@127.0.0.1:25432/postgres")
                .await?;
        Ok(Connector { pg_pool })
    }
}
