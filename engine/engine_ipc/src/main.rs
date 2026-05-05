pub mod server;
#[tokio::main]
pub async fn main() {
    use server::Server;
    let server = Server::new("127.0.0.1:9000".to_string()).await;
    server.run().await;
}
