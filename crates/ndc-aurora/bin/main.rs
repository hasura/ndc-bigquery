use ndc_aurora::connector::Aurora;
use ndc_sdk::default_main::default_main;

#[tokio::main]
pub async fn main() {
    default_main::<Aurora>().await.unwrap()
}
