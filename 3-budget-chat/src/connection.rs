use std::sync::{Arc, RwLock};

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::broadcast;

use crate::server::ChatEvent;

pub struct Connection {
    socket_rx: BufReader<OwnedReadHalf>,
    socket_tx: OwnedWriteHalf,
    joined_users: Arc<RwLock<Vec<String>>>,
    chat_tx_channel: broadcast::Sender<ChatEvent>,
    chat_rx_channel: broadcast::Receiver<ChatEvent>,
}

impl Connection {
    pub fn new(
        socket: TcpStream,
        joined_users: Arc<RwLock<Vec<String>>>,
        chat_tx_channel: broadcast::Sender<ChatEvent>,
        chat_rx_channel: broadcast::Receiver<ChatEvent>,
    ) -> Self {
        let (socket_rx, socket_tx) = socket.into_split();
        Self {
            socket_rx: BufReader::new(socket_rx),
            socket_tx,
            joined_users,
            chat_tx_channel,
            chat_rx_channel,
        }
    }

    pub async fn handle(mut self) -> Result<()> {
        let greetings_msg = "Welcome to budgetchat! What shall I call you?\n";
        self.socket_tx.write_all(greetings_msg.as_bytes()).await?;
        let mut username = String::new();
        self.socket_rx.read_line(&mut username).await?;
        let self_username = username.trim_end().to_string();

        self.send_joined_users().await?;
        self.join_user(&self_username);

        let mut messages = self.socket_rx.lines();
        loop {
            tokio::select! {
                line = messages.next_line() => {
                    match line {
                        Ok(Some(message)) => {
                            self.chat_tx_channel
                                .send(ChatEvent::Message{username: self_username.to_owned(), message})
                                .unwrap(); // Can't fail as we also hold one receiver
                        },
                        // If we receive invalid UTF-8 or EOF, the user leaves the chat
                        _ => break,
                    }
                }
                Ok(event) = self.chat_rx_channel.recv() => {
                    match event {
                        ChatEvent::UserJoined(username) => {
                            if username != self_username {
                                self.socket_tx.write_all(
                                    format!("* {username} has entered the room\n").as_bytes()
                                ).await?;
                            }
                        },
                        ChatEvent::UserLeft(username) => {
                            if username != self_username {
                                self.socket_tx.write_all(
                                    format!("* {username} has left the room\n").as_bytes()
                                ).await?;
                            }
                        },
                        ChatEvent::Message { username, message } => {
                            if username != self_username {
                                self.socket_tx.write_all(
                                    format!("[{username}] {message}\n").as_bytes()
                                ).await?;
                            }
                        },
                    };
                }
            }
        }

        // User is leaving the chat. We can ignore the error if any.
        let _ = self
            .chat_tx_channel
            .send(ChatEvent::UserLeft(self_username));
        Ok(())
    }

    /// Send list of currently joined users
    async fn send_joined_users(&mut self) -> Result<()> {
        let all_users_list = self.joined_users.read().unwrap().join(", ");
        self.socket_tx
            .write_all(format!("* Chatting now: {all_users_list}\n").as_bytes())
            .await?;
        Ok(())
    }

    /// Add username to the list of users in the chat room
    /// and send to all other users that this one have joined
    fn join_user(&self, username: &str) {
        let mut users = self.joined_users.write().unwrap();
        users.push(username.to_owned());

        self.chat_tx_channel
            .send(ChatEvent::UserJoined(username.to_owned()))
            .unwrap();
    }
}
