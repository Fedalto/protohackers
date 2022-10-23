use std::net::SocketAddr;

use anyhow::Result;
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::connection::handle_new_connection;

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub async fn new(address: impl ToSocketAddrs) -> Result<Self> {
        Ok(Server {
            listener: TcpListener::bind(address).await?,
        })
    }

    /// Returns the local address that the Server is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr().unwrap()
    }

    pub async fn run(self) {
        loop {
            let (socket, address) = self.listener.accept().await.unwrap();
            info!("New client connected from {address}");

            tokio::spawn(async move {
                if let Err(err) = handle_new_connection(socket, address).await {
                    error!("{address} Error: {err}");
                }
            });
        }
    }
}
