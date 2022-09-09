#[macro_use]
extern crate log;

use std::io;

use pretty_env_logger::env_logger::Env;
use tokio::io::copy;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    let bind_address = "0.0.0.0:10007";
    let listener = TcpListener::bind(bind_address).await?;
    info!("Starting ECHO server at {bind_address}");

    loop {
        let (mut stream, address) = listener.accept().await?;
        info!("New connection from {address}");
        tokio::spawn(async move {
            let (mut reader, mut writer) = stream.split();
            copy(&mut reader, &mut writer).await?;
            Ok::<_, io::Error>(())
        });
    }
}
