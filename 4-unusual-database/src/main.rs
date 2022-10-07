use std::collections::HashMap;

use anyhow::Result;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<()> {
    let address = "0.0.0.0:9004";
    let socket = UdpSocket::bind(address).await?;
    let mut storage = HashMap::new();
    let mut buffer = [0u8; 1000];

    loop {
        let (len, client) = socket.recv_from(&mut buffer).await?;
        let message = String::from_utf8(buffer[..len].to_vec())?;
        match message.split_once('=') {
            // Query
            None => {
                if message == "version" {
                    socket
                        .send_to("version=Ken's Key-Value Store 1.0".as_bytes(), client)
                        .await?;
                } else {
                    let value = storage.get(&message).cloned().unwrap_or_default();
                    socket
                        .send_to(format!("{message}={value}").as_bytes(), client)
                        .await?;
                }
            }

            // Insert
            Some((key, value)) => {
                if key == "version" {
                    continue;
                }
                storage.insert(key.to_owned(), value.to_owned());
            }
        }
    }
}
