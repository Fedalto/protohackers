use anyhow::Result;
use tokio::net::TcpListener;

use crate::connection;

pub async fn run(listener: TcpListener) -> Result<()> {
    loop {
        let (stream, address) = listener.accept().await?;

        info!("New connection from {}", address);
        tokio::spawn(async move {
            connection::handle_connection(stream).await?;
            Ok::<(), anyhow::Error>(())
        });
    }
}
