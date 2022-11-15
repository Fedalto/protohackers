use std::net::SocketAddr;

use anyhow::{bail, Result};
use bytes::BytesMut;
use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

use crate::frame::{ClientFrame, ServerFrame};
use crate::heartbeat::Heartbeat;
use crate::road_map::{IslandMap, Plate, ProcessorCommand};

pub(crate) async fn handle_new_connection(
    socket: TcpStream,
    address: SocketAddr,
    map: IslandMap,
) -> Result<()> {
    let mut connection = ConnectionHandler::new(socket, address);
    let mut heartbeat = Heartbeat::default();

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                // TODO: Send the write_channel to the Heartbeat and create a tokio task for it?
                connection.write_channel.send(ServerFrame::Heartbeat).await.unwrap();
            }
            frame_res = connection.read_frame() => {
                let frame = match frame_res {
                    Ok(frame) => frame,
                    Err(err) => {
                        let _ = connection.write_channel.send(ServerFrame::Error(b"Error reading frame".to_vec())).await;
                        bail!("Error reading frame. {err}");
                    }
                };
                match frame {
                    // Reached EOF
                    None => return Ok(()),
                    Some(ClientFrame::IAmCamera { road, mile, limit }) => {
                        let road_channel = map.get_or_create_road(road, limit);
                        connection.set_client_type(ClientType::Camera {
                            road,
                            mile,
                            road_channel,
                        }).await?;
                    }
                    Some(ClientFrame::IAmDispatcher { roads, .. }) => {
                        connection.set_client_type(ClientType::Dispatcher { roads: roads.clone() }).await?;
                        let (tx, rx) = oneshot::channel();
                        map.ticket_processor.send(ProcessorCommand::NewDispatcher {
                            roads, ch: tx
                        }).await.unwrap();
                        let ticket_channels = rx.await.unwrap();
                        for ch in ticket_channels {
                            let write_channel = connection.write_channel.clone();
                            tokio::spawn(async move {
                                loop {
                                    let ticket = ch.recv().await.unwrap();
                                    debug!("Dispatching new ticket. ticket={:?}", ticket);
                                    let ticket_frame = ServerFrame::Ticket {
                                        plate: ticket.plate,
                                        road: ticket.road,
                                        mile1: ticket.mile1,
                                        timestamp1: ticket.timestamp1,
                                        mile2: ticket.mile2,
                                        timestamp2: ticket.timestamp2,
                                        speed: ticket.speed,
                                    };
                                    write_channel.send(ticket_frame).await.unwrap();  // FIXME
                                }
                            });
                        }
                    }
                    Some(ClientFrame::Plate { plate, timestamp }) => match connection.client_type {
                        Some(ClientType::Camera {
                            mile,
                            ref road_channel,
                            ..
                        }) => {
                            let plate = Plate::new(plate, mile, timestamp);
                            road_channel.send(plate).await.unwrap();
                        }
                        _ => {
                            connection.error("Received plate, but client did not registered as a camera before").await;
                            bail!("Received plate, but client did not registered as a camera before");
                        }
                    },
                    Some(ClientFrame::WantHeartbeat { interval }) => {
                        if let Err(err) = heartbeat.set_interval(interval) {
                            connection.error(&err.to_string()).await;
                        };
                    }
                };
            }
        }
    }
}

enum ClientType {
    Camera {
        road: u16,
        mile: u16,
        road_channel: mpsc::Sender<Plate>,
    },
    Dispatcher {
        roads: Vec<u16>,
    },
}

struct ConnectionHandler {
    address: SocketAddr,
    read_socket: BufReader<OwnedReadHalf>,
    write_channel: mpsc::Sender<ServerFrame>,
    buffer: BytesMut,
    client_type: Option<ClientType>,
}

impl ConnectionHandler {
    pub fn new(socket: TcpStream, address: SocketAddr) -> Self {
        let (read, write) = socket.into_split();
        let (write_channel_tx, write_channel_rx) = mpsc::channel(16);
        tokio::spawn(write_frame(write_channel_rx, write));

        Self {
            address,
            read_socket: BufReader::new(read),
            write_channel: write_channel_tx,
            buffer: BytesMut::new(),
            client_type: None,
        }
    }

    pub async fn set_client_type(&mut self, client_type: ClientType) -> Result<()> {
        if self.client_type.is_none() {
            self.client_type = Some(client_type)
        } else {
            self.write_channel
                .send(ServerFrame::Error(b"Client already registered".to_vec()))
                .await?;
            bail!("Client already registered");
        }
        Ok(())
    }

    /// Read the next frame from the connection stream
    ///
    /// # Cancel safety
    /// This is cancel safe.
    pub async fn read_frame(&mut self) -> Result<Option<ClientFrame>> {
        ClientFrame::parse(&mut self.read_socket).await
        // loop {
        //     if let Some(frame) = self.parse_frame() {
        //         return Ok(Some(frame));
        //     }
        //
        //     // FIXME: This is causing a panic and I don't know why for sure.
        //     // Maybe removing the self.buffer helps?
        //     // It shouldn't be needed as `self.read_socket` is already buffered.
        //     if 0 == self.read_socket.read_buf(&mut self.buffer).await.unwrap() {
        //         // Reached EOF
        //         return Ok(None);
        //     }
        // }
    }

    // fn parse_frame(&mut self) -> Option<ClientFrame> {
    //     let mut buffer = Cursor::new(&self.buffer[..]);
    //
    //     if ClientFrame::check(&mut buffer) {
    //         buffer.set_position(0);
    //         let frame = ClientFrame::parse(&mut buffer)
    //             .context(format!("Client {}", self.address))
    //             .unwrap();
    //         self.buffer.advance(buffer.position() as usize);
    //         return Some(frame);
    //     }
    //     None
    // }

    pub async fn error(&self, error_msg: &str) {
        self.write_channel
            .send(ServerFrame::Error(error_msg.as_bytes().to_vec()))
            .await
            .unwrap();
    }
}

async fn write_frame(
    mut channel: mpsc::Receiver<ServerFrame>,
    socket: OwnedWriteHalf,
) -> Result<()> {
    let mut socket = BufWriter::new(socket);
    loop {
        match channel.recv().await {
            Some(ServerFrame::Error(err)) => {
                socket.write_u8(0x10).await?;
                socket.write_u8(err.len() as u8).await?;
                socket.write_all(&err).await?;
                socket.flush().await?;
            }
            Some(ServerFrame::Ticket {
                plate,
                road,
                mile1,
                timestamp1,
                mile2,
                timestamp2,
                speed,
            }) => {
                socket.write_u8(0x21).await?;
                socket.write_u8(plate.len() as u8).await?;
                socket.write_all(&plate).await?;
                socket.write_u16(road).await?;
                socket.write_u16(mile1).await?;
                socket.write_u32(timestamp1).await?;
                socket.write_u16(mile2).await?;
                socket.write_u32(timestamp2).await?;
                socket.write_u16(speed).await?;
                socket.flush().await?;
            }
            Some(ServerFrame::Heartbeat) => {
                socket.write_u8(0x41).await?;
                socket.flush().await?;
            }
            // Connection was closed
            None => break,
        };
    }
    Ok(())
}
