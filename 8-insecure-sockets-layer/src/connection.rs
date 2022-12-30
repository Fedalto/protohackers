use std::net::SocketAddr;

use anyhow::bail;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufStream};
use tokio::net::TcpStream;
use tracing::{info, instrument};

use crate::cipher_stream::CipherStream;
use crate::ciphers::{AddN, AddPos, Cipher, ReverseBits, XorN, XorPos};
use crate::toy_workshop::prioritise_work;

#[instrument(skip(socket))]
pub async fn handle_new_connection(
    mut socket: TcpStream,
    _address: SocketAddr,
) -> anyhow::Result<()> {
    let cipher_spec = read_cipher_spec(&mut socket).await?;
    let mut cipher_stream = BufStream::new(CipherStream::new(cipher_spec, socket));

    loop {
        let mut toys_request = String::new();
        match cipher_stream.read_line(&mut toys_request).await {
            Ok(0) => {
                // Reached EOF
                return Ok(());
            }
            Ok(_) => {
                info!(?toys_request, "Received line");
                let toys_request = toys_request.trim_end();

                let priority_toy = prioritise_work(toys_request);

                cipher_stream.write_all(priority_toy.as_bytes()).await?;
                cipher_stream.write_all("\n".as_bytes()).await?;
                cipher_stream.flush().await?;
                info!(priority_toy, "Sent response");
            }
            Err(e) => {
                bail!("{e}")
            }
        }
    }
}

#[instrument(skip_all, ret)]
pub async fn read_cipher_spec<R>(src: &mut R) -> anyhow::Result<Vec<Box<dyn Cipher>>>
where
    R: AsyncReadExt + Unpin,
{
    let mut cipher_spec: Vec<Box<dyn Cipher>> = vec![];

    loop {
        match src.read_u8().await? {
            0x0 => break,
            0x1 => cipher_spec.push(Box::new(ReverseBits)),
            0x2 => {
                let n = src.read_u8().await?;
                cipher_spec.push(Box::new(XorN::new(n)));
            }
            0x3 => cipher_spec.push(Box::new(XorPos)),
            0x4 => {
                let n = src.read_u8().await?;
                cipher_spec.push(Box::new(AddN::new(n)))
            }
            0x5 => cipher_spec.push(Box::new(AddPos)),

            invalid => bail!("Invalid cipher spec. byte={invalid}"),
        }
    }
    validate_cipher_spec(&cipher_spec)?;
    Ok(cipher_spec)
}

fn validate_cipher_spec(cipher_spec: &[Box<dyn Cipher>]) -> anyhow::Result<()> {
    let test_buf = "abcd".as_bytes().to_vec();
    let ciphered_buf = cipher_spec
        .iter()
        .fold(test_buf.clone(), |buf, cipher| cipher.apply(&buf, 2));
    if ciphered_buf == test_buf {
        bail!("no-op cipher spec provided")
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_cipher_spec() {
        let cipher_spec: Vec<Box<dyn Cipher>> = vec![Box::new(XorN::new(0))];
        assert!(validate_cipher_spec(&cipher_spec).is_err());

        let cipher_spec: Vec<Box<dyn Cipher>> = vec![Box::new(ReverseBits), Box::new(ReverseBits)];
        assert!(validate_cipher_spec(&cipher_spec).is_err());

        let cipher_spec: Vec<Box<dyn Cipher>> = vec![
            Box::new(XorN::new(0xa0)),
            Box::new(XorN::new(0x0b)),
            Box::new(XorN::new(0xab)),
        ];
        assert!(validate_cipher_spec(&cipher_spec).is_err());
    }
}
