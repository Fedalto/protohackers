use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::broadcast;

use crate::connection::Connection;

#[derive(Debug, Clone)]
pub enum ChatEvent {
    Message { username: String, message: String },
    UserJoined(String),
    UserLeft(String),
}

pub struct Server {
    listener: TcpListener,
    joined_users: Arc<RwLock<Vec<String>>>,
    chat_tx_channel: broadcast::Sender<ChatEvent>,
}

impl Server {
    pub async fn new(bind_address: impl ToSocketAddrs) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_address).await?;
        let (tx, _rx) = broadcast::channel(16);
        Ok(Self {
            listener,
            joined_users: Arc::new(RwLock::new(Vec::new())),
            chat_tx_channel: tx,
        })
    }

    pub async fn run(self) {
        loop {
            let (socket, address) = self.listener.accept().await.unwrap();
            info!("New connection from {address}");

            let joined_users = Arc::clone(&self.joined_users);
            let tx = self.chat_tx_channel.clone();
            let rx = self.chat_tx_channel.subscribe();
            tokio::spawn(async {
                let connection = Connection::new(socket, joined_users, tx, rx);
                connection.handle().await
            });
        }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}
