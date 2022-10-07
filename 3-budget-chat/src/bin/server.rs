use anyhow::Result;
use log::info;

use budget_chat::server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let address = "0.0.0.0:9003";
    let server = Server::new(address).await?;

    info!("Start Budget Chat at {address}");
    server.run().await;
    Ok(())
}
