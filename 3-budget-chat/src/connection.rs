use anyhow::{anyhow, bail, Result};
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct Connection {
    socket: TcpStream,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(socket: TcpStream) -> Self {
        Self {
            socket,
            buffer: BytesMut::new(),
        }
    }

    pub async fn handle(mut self) -> Result<()> {
        let greetings_msg = "Welcome to budgetchat! What shall I call you?";
        self.socket.write_all(greetings_msg.as_bytes()).await?;
        let username = self.read_frame().await?.ok_or(anyhow!("Disconnected"))?;

        // TODO: Join user to current session
        // TODO: Send to all other users that this one have joined
        // TODO: Send the list of connected users to this new user

        // Loop for the next messages
        while let Some(frame) = self.read_frame().await? {}

        // Client closed the connection
        // TODO: Remove user from session
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
