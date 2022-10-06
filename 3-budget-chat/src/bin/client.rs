use anyhow::Result;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    let address = "127.0.0.1:9003";
    let mut connection = TcpStream::connect(address).await?;

    Ok(())
}
