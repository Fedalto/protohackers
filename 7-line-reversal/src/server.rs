use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use tokio::net::{ToSocketAddrs, UdpSocket};
use tokio::sync::mpsc;

use crate::message::{Message, SessionId};
use crate::session::Session;

pub struct Server {
    socket: Arc<UdpSocket>,
    sessions: HashMap<SessionId, mpsc::Sender<Message>>,
}

impl Server {
    pub async fn new(address: impl ToSocketAddrs) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(address).await?;
        Ok(Self {
            socket: Arc::new(socket),
            sessions: HashMap::new(),
        })
    }

    pub async fn run(&mut self) -> Result<(), io::Error> {
        let mut buffer = vec![0u8; 1000];

        loop {
            let (len, peer_address) = self.socket.recv_from(&mut buffer).await?;
            match Message::try_from(&buffer[0..len]) {
                Err(err) => {
                    warn!(
                        "Received invalid packet: err={:?}, packet={:?}",
                        err,
                        &buffer[0..len]
                    );
                    continue;
                }
                Ok(message) => {
                    let session_id = message.session_id();
                    let tx = self.sessions.entry(session_id).or_insert_with(|| {
                        let (tx, rx) = mpsc::channel(16);
                        let mut session =
                            Session::new(self.socket.clone(), session_id, peer_address, rx);
                        tokio::spawn(async move { session.run().await });
                        tx
                    });
                    if let Err(_) = tx.send(message).await {
                        error!(
                            "Error sending message to Session. Session was already disconnected. peer_addr={peer_address}"
                        );
                        let _ = self
                            .socket
                            .send_to(&Message::Disconnect(session_id).to_vec(), peer_address)
                            .await;
                    }
                }
            };
        }
    }
}
