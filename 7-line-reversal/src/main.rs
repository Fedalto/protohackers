use line_reversal::server::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let address = "0.0.0.0:9007";
    let mut server = Server::new(address).await?;
    server.run().await?;
    Ok(())
}
