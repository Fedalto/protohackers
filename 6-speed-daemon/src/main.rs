use std::net::Ipv4Addr;

use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::prelude::*;
use tracing_subscriber::Layer;

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

fn convert_level_filter(filter: log::LevelFilter) -> tracing_subscriber::filter::LevelFilter {
    match filter {
        log::LevelFilter::Off => tracing_subscriber::filter::LevelFilter::OFF,
        log::LevelFilter::Error => tracing_subscriber::filter::LevelFilter::ERROR,
        log::LevelFilter::Warn => tracing_subscriber::filter::LevelFilter::WARN,
        log::LevelFilter::Info => tracing_subscriber::filter::LevelFilter::INFO,
        log::LevelFilter::Debug => tracing_subscriber::filter::LevelFilter::DEBUG,
        log::LevelFilter::Trace => tracing_subscriber::filter::LevelFilter::TRACE,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli: Cli = Cli::parse();
    let subscriber = tracing_subscriber::fmt::layer()
        .with_filter(convert_level_filter(cli.verbose.log_level_filter()));
    tracing_subscriber::registry().with(subscriber).init();

    let bind_address = (cli.host, cli.port);
    let listener = TcpListener::bind(bind_address).await?;
    let server = Server::new(listener);
    info!(address = server.local_addr().to_string(), "Starting server");
    server.run().await;

    Ok(())
}
