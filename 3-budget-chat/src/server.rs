use std::io;
use std::net::SocketAddr;

use tokio::net::{TcpListener, ToSocketAddrs};

use crate::connection::Connection;

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub async fn new(bind_address: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_address).await?;
        Ok(Self { listener })
    }

    pub async fn run(self) {
        while let (socket, address) = self.listener.accept().await.unwrap() {
            info!("New connection from {address}");
            tokio::spawn(async {
                let connection = Connection::new(socket);
                connection.handle().await
            });
        }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}
