use std::net::Ipv4Addr;

use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use log::info;

use line_reversal::server::Server;

#[derive(Parser)]
struct Cli {
    #[clap(long, default_value_t = Ipv4Addr::from(0))]
    pub host: Ipv4Addr,

    #[clap(short, long, default_value_t = 9006)]
    pub port: u16,

    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli: Cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .parse_default_env()
        .init();

    let address = (cli.host, cli.port);
    let mut server = Server::new(address).await?;
    info!("Starting server at {}", server.local_addr());
    server.run().await?;
    Ok(())
}
