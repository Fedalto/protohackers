use std::net::Ipv4Addr;

use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use log::info;
use tokio::net::TcpListener;

use speed_daemon::server::Server;

#[derive(Parser, Debug)]
struct Cli {
    #[clap(long, default_value_t = Ipv4Addr::from(0))]
    pub host: Ipv4Addr,

    #[clap(short, long, default_value_t = 9006)]
    pub port: u16,

    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli: Cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .parse_default_env()
        .init();

    let bind_address = (cli.host, cli.port);
    let listener = TcpListener::bind(bind_address).await?;
    let server = Server::new(listener);
    info!("Starting server at {}", server.local_addr());
    server.run().await;

    Ok(())
}
