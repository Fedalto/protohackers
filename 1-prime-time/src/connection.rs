use anyhow::{anyhow, Result};
use bytes::{Buf, BytesMut};
use std::io::Cursor;
use tokio::io::{AsyncReadExt, BufWriter};
use tokio::net::TcpStream;

use crate::frame::Frame;

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

    pub async fn write_frame(&mut self, frame: Frame) {
        todo!()
    }
    fn parse_frame(&mut self) -> Option<Frame> {
        if let Some((frame, len)) = Frame::parse(&self.buffer) {
            self.buffer.advance(len);
            return Some(frame);
        }
        None
    }
}

pub async fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut connection = Connection::new(stream);

    loop {
        let new_frame = connection.read_frame().await?;
    }
}
