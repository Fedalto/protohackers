use anyhow::Result;

use budget_chat::server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    let address = "0.0.0.0:9003";
    // let listener = TcpListener::bind(address).await?;
    let server = Server::new(address).await?;

    server.run().await;
    Ok(())
}
