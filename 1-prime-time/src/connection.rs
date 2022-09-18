use anyhow::{anyhow, Result};
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

use crate::frame::Frame;
use crate::handler::handle;

struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: BufWriter::new(stream),
            buffer: BytesMut::new(),
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            if let Some(new_frame) = self.parse_frame() {
                return Ok(Some(new_frame));
            }

            if self.stream.read_buf(&mut self.buffer).await? == 0 {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(anyhow!("Connection reset by peer"));
                }
            }
        }
    }

    pub async fn write_frame(&mut self, frame: Frame) -> Result<()> {
        self.stream.write_all(&frame.0).await?;
        self.stream.write_u8(b'\n').await?;
        self.stream.flush().await?;

        Ok(())
    }

    fn parse_frame(&mut self) -> Option<Frame> {
        if let Some((frame, len)) = Frame::parse(&self.buffer) {
            self.buffer.advance(len);
            return Some(frame);
        }
        None
    }
}

pub async fn handle_connection(stream: TcpStream) -> Result<()> {
    let mut connection = Connection::new(stream);

    while let Some(new_frame) = connection.read_frame().await? {
        match handle(new_frame.0) {
            Ok(response) => {
                let frame = Frame::from(response);
                connection.write_frame(frame).await?;
            }
            Err(err) => {
                // An error happened while handling the request.
                // A malformed response must be sent back and the connection terminated
                let error_message = format!("{{\"error\":\"{}\"}}", err);
                let error_frame = Frame::from(error_message.as_bytes());
                connection.write_frame(error_frame).await?;
                break;
            }
        }
    }

    Ok(())
}
