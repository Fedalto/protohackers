use anyhow::Result;
use tokio::net::TcpListener;

use speed_daemon::server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let listener = TcpListener::bind("0.0.0.0:9006").await?;
    let server = Server::new(listener);
    server.run().await;

    Ok(())
}
