extern crate core;
#[macro_use]
extern crate log;

use anyhow::Result;
use pretty_env_logger::env_logger::Env;
use tokio::net::TcpListener;

use prime_time::server;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    let bind_address = "0.0.0.0:9901";
    let listener = TcpListener::bind(bind_address).await?;
    info!("Starting server at {bind_address}");

    Ok(server::run(listener).await?)
}
