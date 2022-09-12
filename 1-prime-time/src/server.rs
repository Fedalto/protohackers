use crate::connection;
use anyhow::Result;
use tokio::net::TcpListener;

pub async fn run(listener: TcpListener) -> Result<()> {
    loop {
        let (stream, _address) = listener.accept().await?;

        tokio::spawn(async move {
            connection::handle_connection(stream).await?;
            Ok::<(), anyhow::Error>(())
        });
    }
}
