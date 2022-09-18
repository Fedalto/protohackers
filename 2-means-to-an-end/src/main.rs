use anyhow::Result;
use tokio::net::TcpListener;

use means_to_an_end::server::run;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let listener = TcpListener::bind("0.0.0.0:9902").await?;

    run(listener).await
}
