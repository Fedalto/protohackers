use std::net::SocketAddr;

use tokio::net::TcpListener;

use crate::connection::handle_new_connection;
use crate::road_map::IslandMap;

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
        let island_map = IslandMap::new();
        loop {
            let (socket, address) = self.listener.accept().await.unwrap();
            let map = island_map.clone();
            tokio::spawn(async move {
                if let Err(error_message) = handle_new_connection(socket, address, map).await {
                    error!(%error_message, %address, "Client disconnect");
                };
            });
        }
    }
}
