use std::io;
use std::io::Write;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<()> {
    let address = "127.0.0.1:9003";
    let mut connection = TcpStream::connect(address).await?;
    let (rx, mut tx) = connection.split();

    let mut received_lines = BufReader::new(rx).lines();
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines_to_send = stdin.lines();

    loop {
        tokio::select! {
            line = received_lines.next_line() => {
                println!("{}", line.unwrap().unwrap());
                io::stdout().flush().unwrap();
            }
            line = lines_to_send.next_line() => {
                tx.write_all(format!("{}\n", line.unwrap().unwrap()).as_bytes()).await?;
            }
        }
    }
}
