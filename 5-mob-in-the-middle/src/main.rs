use std::net::Ipv4Addr;

use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use log::info;

use mob_in_the_middle::server::Server;

#[derive(Parser, Debug)]
struct Args {
    /// Host to bind to
    #[arg(short = 'H', long, default_value_t = Ipv4Addr::from(0))]
    pub host: Ipv4Addr,

    /// Port to listen
    #[arg(short, long, default_value_t = 9005)]
    pub port: u16,

    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Args = Args::parse();
    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .parse_default_env()
        .init();

    let bind_address = (args.host, args.port);
    let server = Server::new(bind_address).await?;
    info!("Proxy started at {}", server.local_addr());
    server.run().await;

    Ok(())
}
