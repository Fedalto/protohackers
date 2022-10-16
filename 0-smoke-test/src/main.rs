#[macro_use]
extern crate log;

use std::io;
use std::net::Ipv4Addr;

use clap::Parser;
use pretty_env_logger::env_logger::Env;
use tokio::io::copy;
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
#[command(about)]
struct Args {
    /// Host address to bind to
    #[arg(default_value_t = Ipv4Addr::from(0))]
    pub host: Ipv4Addr,

    /// Port to listen
    #[arg(default_value_t = 7)]
    pub port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Args = Args::parse();

    pretty_env_logger::env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .init();

    let bind_address = (args.host, args.port);
    let listener = TcpListener::bind(bind_address).await?;
    info!("Starting ECHO server at {}", listener.local_addr().unwrap());

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
