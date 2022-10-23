use std::borrow::Cow;
use std::net::SocketAddr;

use anyhow::Result;
use fancy_regex::Regex;
use lazy_static::lazy_static;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

const TONY_ADDRESS: &str = "7YWHMfk9JZe0LM0g1ZauHuiSxhI";
const UPSTREAM: &str = "chat.protohackers.com:16963";

pub async fn handle_new_connection(mut socket: TcpStream, address: SocketAddr) -> Result<()> {
    let mut upstream_socket = TcpStream::connect(UPSTREAM).await?;
    let (upstream_rx, mut upstream_tx) = upstream_socket.split();
    let upstream_rx_buffer = BufReader::new(upstream_rx);
    let mut upstream_lines = upstream_rx_buffer.lines();

    let (client_rx, mut client_tx) = socket.split();
    let mut client_rx_buffer = BufReader::new(client_rx);
    let mut incoming_buffer = Vec::new();

    loop {
        tokio::select! {
            res = client_rx_buffer.read_until(b'\n', &mut incoming_buffer) => {
                match res {
                    // EOF
                    Ok(0) => break,
                    Ok(_) => {
                        let line = String::from_utf8(incoming_buffer.clone())?;
                        debug!("{address} Received: {}", line.trim_end());
                        let intercepted_line = rewrite_boguscoin_address(&line);
                        upstream_tx.write_all(intercepted_line.as_bytes()).await?;
                        incoming_buffer.clear();
                    }
                    _ => break,
                }
            }
            line = upstream_lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        let intercepted_line = rewrite_boguscoin_address(&line);
                        client_tx.write_all(intercepted_line.as_bytes()).await?;
                        client_tx.write_all(b"\n").await?;
                    }
                    _ => break,
                }
            }
        }
    }
    info!("{address} disconnected.");
    Ok(())
}

fn rewrite_boguscoin_address(input: &str) -> Cow<str> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?<= |^)7[\d\w]{25,34}(?= |$|\n)").unwrap();
    }

    RE.replace_all(input, TONY_ADDRESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite() {
        let input = "Please send the payment of 750 Boguscoins to 7OchKZQaPhMmEqFozQONUv8eJTFH";
        let output = rewrite_boguscoin_address(input);
        assert_eq!(
            "Please send the payment of 750 Boguscoins to 7YWHMfk9JZe0LM0g1ZauHuiSxhI",
            output,
        );

        let input = "7huzN5goJ9RzEPw2RXtHrJb1STG 76cxq1UBIH65j5MVMz8WbfoCe7mZyf22QN 7gMK6ZtnJe9jtvkk36pLii3VCr";
        let output = rewrite_boguscoin_address(input);
        assert_eq!(
            output,
            "7YWHMfk9JZe0LM0g1ZauHuiSxhI 7YWHMfk9JZe0LM0g1ZauHuiSxhI 7YWHMfk9JZe0LM0g1ZauHuiSxhI",
        );

        let input = "7YIdMCoDobwYMNnKCuNY3jsHlVODM-MuuHTkrmzDvcqUZqRBiBCxjooXyqQEWK5IJ-1234";
        assert_eq!(input, rewrite_boguscoin_address(input));
    }
}
