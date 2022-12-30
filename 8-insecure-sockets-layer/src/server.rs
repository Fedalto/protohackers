use std::net::SocketAddr;

use tokio::net::TcpListener;
use tracing::error;

use crate::connection::handle_new_connection;

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub fn new(listener: TcpListener) -> Self {
        Self { listener }
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr().unwrap()
    }

    pub async fn run(self) {
        loop {
            let (socket, address) = self.listener.accept().await.unwrap();
            tokio::spawn(async move {
                if let Err(e) = handle_new_connection(socket, address).await {
                    error!(?address, "Connection closed: {e}");
                }
            });
        }
    }
}
