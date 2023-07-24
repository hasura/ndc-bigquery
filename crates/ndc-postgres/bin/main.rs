use ndc_hub::default_main::default_main;
use ndc_postgres::connector::Postgres;

#[tokio::main]
pub async fn main() {
    default_main::<Postgres>().await.unwrap()
}
