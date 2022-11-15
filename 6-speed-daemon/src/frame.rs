use std::io::ErrorKind;

use anyhow::{anyhow, bail, Result};
use tokio::io::{AsyncBufRead, AsyncReadExt};

#[derive(Debug)]
pub(crate) enum ClientFrame {
    Plate { plate: Vec<u8>, timestamp: u32 },
    WantHeartbeat { interval: u32 },
    IAmCamera { road: u16, mile: u16, limit: u16 },
    IAmDispatcher { num_roads: u8, roads: Vec<u16> },
}

impl ClientFrame {
    pub async fn parse<R: AsyncBufRead + Unpin>(src: &mut R) -> Result<Option<ClientFrame>> {
        let frame = match src.read_u8().await {
            // Plate
            Ok(0x20) => {
                let plate = get_str(src).await?;
                let timestamp = src.read_u32().await?;
                ClientFrame::Plate {
                    plate: plate.to_vec(),
                    timestamp,
                }
            }
            // WantHeartbeat
            Ok(0x40) => ClientFrame::WantHeartbeat {
                interval: src.read_u32().await?,
            },
            // IAmCamera
            Ok(0x80) => {
                let road = src.read_u16().await?;
                let mile = src.read_u16().await?;
                let limit = src.read_u16().await?;
                ClientFrame::IAmCamera { road, mile, limit }
            }
            // IAmDispatcher
            Ok(0x81) => {
                let num_roads = src.read_u8().await?;
                let mut roads = Vec::with_capacity(num_roads as usize);
                for _ in 0..num_roads {
                    roads.push(src.read_u16().await?);
                }
                ClientFrame::IAmDispatcher { num_roads, roads }
            }

            Err(err) => {
                // Reached EOF
                return if err.kind() == ErrorKind::UnexpectedEof {
                    Ok(None)
                } else {
                    Err(anyhow!(err))
                };
            }

            // Unknown message type. Return error
            _e => bail!("Unknown error: {_e:?}"),
        };
        log::debug!("Parsed new frame: {:?}", frame);
        Ok(Some(frame))
    }
}

async fn get_str<R: AsyncBufRead + Unpin>(src: &mut R) -> Result<Vec<u8>> {
    let str_len = src.read_u8().await?;
    let mut string = Vec::with_capacity(str_len as usize);
    for _ in 0..str_len {
        string.push(src.read_u8().await?);
    }
    Ok(string)
}

#[derive(Debug)]
pub(crate) enum ServerFrame {
    Error(Vec<u8>),
    Ticket {
        plate: Vec<u8>,
        road: u16,
        mile1: u16,
        timestamp1: u32,
        mile2: u16,
        timestamp2: u32,
        speed: u16,
    },
    Heartbeat,
}
