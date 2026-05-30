pub mod connection;


use crate::connection::Master;

#[tokio::main]
pub async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("start");
    dotenvy::dotenv().ok();

    let token = std::env::var("TOKEN").expect("TOKEN environment variable not set");

    let master = Master::new(token).await;
    master.start().await;
}
