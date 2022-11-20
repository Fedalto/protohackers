use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::sync::{broadcast, mpsc};

use crate::message::{Message, SessionId};

enum Error {
    Disconnect,
}

pub struct Session {
    id: SessionId,
    socket: Arc<UdpSocket>,
    peer_address: SocketAddr,
    rx: mpsc::Receiver<Message>,
    timeout_tx: broadcast::Sender<()>,

    /// If the peer have sent the `connect` message before sending other messages
    has_connected: bool,

    /// Buffer of data received so far
    data: String,
    /// How many bytes we received in this session so far.
    /// Used when receiving new `data`
    bytes_received: u32,

    /// How many bytes was sent to the peer
    bytes_sent: Arc<RwLock<u32>>,
    /// How many bytes the peer have already acked
    bytes_acked: Arc<RwLock<u32>>,
}

impl Session {
    pub fn new(
        socket: Arc<UdpSocket>,
        session_id: SessionId,
        peer_address: SocketAddr,
        rx: mpsc::Receiver<Message>,
    ) -> Self {
        let (timeout_tx, _) = broadcast::channel(1);

        Self {
            id: session_id,
            socket,
            peer_address,
            rx,
            timeout_tx,
            has_connected: false,
            bytes_received: 0,
            bytes_sent: Arc::new(RwLock::new(0)),
            bytes_acked: Arc::new(RwLock::new(0)),
            data: String::new(),
        }
    }

    pub async fn run(&mut self) {
        info!(
            "New session connected. session_id={}, peer_address={}",
            self.id, self.peer_address
        );
        let mut timeout_rx = self.timeout_tx.subscribe();
        loop {
            tokio::select! {
                _ = timeout_rx.recv() => {
                    info!("Session timed out. session_id={}, peer_address={}", self.id, self.peer_address);
                    return;
                }

                Some(message) = self.rx.recv() => {
                    if self.handle_new_message(message).await.is_err() {
                        info!("Session closed. session={}, peer_address={}", self.id, self.peer_address);
                    };
                }
            }
        }
    }

    async fn handle_new_message(&mut self, message: Message) -> Result<(), Error> {
        info!(
            "Received new message. session_id={}, message={:?}",
            self.id, message
        );
        match message {
            Message::Connect(_) => {
                self.has_connected = true;
                self.ack(0).await;
            }

            Message::Data { position, data, .. } => {
                if !self.has_connected {
                    self.close().await?;
                }

                if position > self.bytes_received {
                    self.ack(self.bytes_received).await;
                } else {
                    let new_data_position = (self.bytes_received - position) as usize;
                    if new_data_position > data.len() {
                        // This message was already seen and the current position is way ahead of it
                        return Ok(());
                    }
                    let new_data = &data[new_data_position..];
                    self.bytes_received += new_data.len() as u32;
                    self.data.push_str(new_data);
                    self.ack(self.bytes_received).await;

                    if new_data.contains('\n') {
                        for line in self
                            .data
                            .split_inclusive("\n")
                            .filter(|l| l.ends_with('\n'))
                        {
                            let reversed_line = reverse_line(line);
                            self.send_line(reversed_line).await;
                        }
                        if let Some(last_str) = self.data.split_inclusive("\n").last() {
                            if last_str.ends_with('\n') {
                                info!("Clearing buffer data. session_id={}", self.id);
                                self.data.clear();
                            } else {
                                info!("Dropping already sent buffer data. session_id={}", self.id);
                                self.data = last_str.to_owned();
                            }
                        }
                    }
                }
            }

            Message::Ack { position, session } => {
                if !self.has_connected {
                    self.close().await?;
                }
                let bytes_sent = { *self.bytes_sent.read().unwrap() };
                if position > bytes_sent {
                    info!("Client sent unexpected ACK. session_id={session}, bytes_sent={bytes_sent}, ack={position}");
                    self.close().await?;
                }
                let mut ack = self.bytes_acked.write().unwrap();
                *ack = position;
            }

            Message::Disconnect(_) => {
                self.close().await?;
            }
        }

        Ok(())
    }

    async fn send_line(&self, line: String) {
        let messages = self.break_line_in_messages(line);
        let timeout = self.timeout_tx.clone();
        tokio::spawn(send_messages(
            self.socket.clone(),
            self.peer_address,
            messages,
            self.bytes_acked.clone(),
            timeout,
        ));
    }

    fn break_line_in_messages(&self, line: String) -> Vec<Message> {
        let mut messages = Vec::new();

        for chunk in line.as_bytes().chunks(970) {
            let data = String::from_utf8_lossy(chunk).to_string();
            let message = {
                let mut position = self.bytes_sent.write().unwrap();
                let message = Message::Data {
                    session: self.id,
                    position: *position,
                    data,
                };
                *position += chunk.len() as u32;
                message
            };
            messages.push(message);
        }
        messages
    }

    async fn ack(&self, position: u32) {
        let ack_message = Message::Ack {
            session: self.id,
            position,
        };
        info!("Sending ACK. {:?}", ack_message);
        self.socket
            .send_to(&ack_message.to_vec(), self.peer_address)
            .await
            .unwrap();
    }

    async fn close(&mut self) -> Result<(), Error> {
        let close_message = Message::Disconnect(self.id);
        self.socket
            .send_to(&close_message.to_vec(), self.peer_address)
            .await
            .unwrap();
        self.rx.close();
        let _ = self.timeout_tx.send(());

        Err(Error::Disconnect)
    }
}

/// Sends the line to peer_address
///
/// Will retry if the peer have not acknowledge and will send a timeout
/// broadcast if the session timeout is reached.
async fn send_messages(
    socket: Arc<UdpSocket>,
    peer_address: SocketAddr,
    messages: Vec<Message>,
    bytes_acked: Arc<RwLock<u32>>,
    timeout: broadcast::Sender<()>,
) {
    let mut session_timeout_rx = timeout.subscribe();
    let mut retransmission_timeout = tokio::time::interval(Duration::from_secs(3));
    let mut session_timeout = tokio::time::interval(Duration::from_secs(20));
    session_timeout.reset();

    loop {
        tokio::select! {
            biased;

            // Some other task send a session timeout. The session is now considered closed.
            _ = session_timeout_rx.recv() => {
                return;
            }

            _ = session_timeout.tick() => {
                let last_ack = { *bytes_acked.read().unwrap() };
                if let Message::Data {position, ..} = messages.last().unwrap() {
                    if last_ack < *position {
                        timeout.send(()).unwrap();
                        return;
                    }
                }
            }

            _ = retransmission_timeout.tick() => {
                let last_ack = { *bytes_acked.read().unwrap() };
                let mut all_messages_acked = true;
                for message in &messages {
                    if let Message::Data {position, ..} = message {
                        if last_ack <= *position {
                            all_messages_acked = false;
                            // debug!("(8) Sending data {:?}", message);
                            socket.send_to(&message.to_vec(), peer_address).await.unwrap();
                        }
                    }
                }
                if all_messages_acked {
                    break;
                }
            }
        }
    }
}

fn reverse_line(line: &str) -> String {
    let mut reversed_line: String = line.trim_end().chars().rev().collect();
    reversed_line.push('\n');
    reversed_line
}
