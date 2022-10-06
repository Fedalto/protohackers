use std::sync::{Arc, RwLock};

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast;

use crate::server::ChatEvent;

pub struct Connection {
    socket: BufReader<TcpStream>,
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
        Self {
            socket: BufReader::new(socket),
            joined_users,
            chat_tx_channel,
            chat_rx_channel,
        }
    }

    pub async fn handle(mut self) -> Result<()> {
        let greetings_msg = "Welcome to budgetchat! What shall I call you?\n";
        self.socket.write_all(greetings_msg.as_bytes()).await?;
        let mut username = String::new();
        self.socket.read_line(&mut username).await?;

        self.send_joined_users().await?;
        self.join_user(&username);

        // Loop for the next messages
        let mut messages = self.socket.lines();
        while let Ok(Some(message)) = messages.next_line().await {
            self.chat_tx_channel
                .send(ChatEvent::Message(message))
                .unwrap(); // Can't fail as we also hold one receiver
        }

        // Client closed the connection
        // TODO: Remove user from session
        Ok(())
    }

    /// Send list of currently joined users
    async fn send_joined_users(&mut self) -> Result<()> {
        let all_users_list = self.joined_users.read().unwrap().join(", ");
        self.socket
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
