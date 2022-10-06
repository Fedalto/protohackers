use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use anyhow::{anyhow, bail, Result};
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::broadcast;

use crate::server::ChatEvent;

pub struct Connection {
    socket: TcpStream,
    buffer: BytesMut,
    joined_users: Arc<RwLock<HashSet<String>>>,
    chat_tx_channel: broadcast::Sender<ChatEvent>,
    chat_rx_channel: broadcast::Receiver<ChatEvent>,
}

impl Connection {
    pub fn new(
        socket: TcpStream,
        joined_users: Arc<RwLock<HashSet<String>>>,
        chat_tx_channel: broadcast::Sender<ChatEvent>,
        chat_rx_channel: broadcast::Receiver<ChatEvent>,
    ) -> Self {
        Self {
            socket,
            buffer: BytesMut::new(),
            joined_users,
            chat_tx_channel,
            chat_rx_channel,
        }
    }

    pub async fn handle(mut self) -> Result<()> {
        let greetings_msg = "Welcome to budgetchat! What shall I call you?\n";
        self.socket.write_all(greetings_msg.as_bytes()).await?;
        let username = self.read_frame().await?.ok_or(anyhow!("Disconnected"))?;

        self.send_joined_users().await?;
        self.join_user(&username);

        // Loop for the next messages
        while let Some(message) = self.read_frame().await? {
            self.chat_tx_channel
                .send(ChatEvent::Message(message))
                .unwrap(); // Can't fail as we also hold a receiver
        }

        // Client closed the connection
        // TODO: Remove user from session
        Ok(())
    }

    /// Send list of currently joined users
    async fn send_joined_users(&mut self) -> Result<()> {
        let all_users_list = self
            .joined_users
            .read()
            .unwrap()
            .iter()
            .fold(String::new(), |a, b| format!("{a}, {b}"));

        self.socket
            .write_all(format!("* Chatting now: {all_users_list}\n").as_bytes())
            .await?;
        Ok(())
    }

    /// Add username to the list of users in the chat room
    /// and send to all other users that this one have joined
    fn join_user(&self, username: &str) -> Result<()> {
        let mut users = self.joined_users.write().unwrap();
        users.insert(username.to_owned());

        self.chat_tx_channel
            .send(ChatEvent::UserJoined(username.to_owned()))?;
        Ok(())
    }

    async fn read_frame(&mut self) -> Result<Option<String>> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            if self.socket.read_buf(&mut self.buffer).await? == 0 {
                // Reached EOF
                return if self.buffer.is_empty() {
                    Ok(None)
                } else {
                    bail!("Connection reset by peer")
                };
            }
        }
    }

    fn parse_frame(&mut self) -> Result<Option<String>> {
        for (i, &byte) in self.buffer.iter().enumerate() {
            if char::from(byte) == '\n' {
                let frame_bytes = self.buffer[0..i - 1].to_vec();
                let frame = String::from_utf8(frame_bytes)?;
                self.buffer.advance(i);
                return Ok(Some(frame));
            }
        }
        Ok(None)
    }
}
