use std::net::SocketAddr;

use anyhow::{bail, Result};
use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

use crate::frame::{ClientFrame, ServerFrame};
use crate::heartbeat::create_heartbeat;
use crate::road_map::{IslandMap, Plate, ProcessorCommand};

pub(crate) async fn handle_new_connection(
    socket: TcpStream,
    address: SocketAddr,
    map: IslandMap,
) -> Result<()> {
    let mut connection = ConnectionHandler::new(socket, address);

    loop {
        let frame = match connection.read_frame().await {
            Ok(frame) => frame,
            Err(err) => {
                let _ = connection
                    .write_channel
                    .send(ServerFrame::Error(b"Error reading frame".to_vec()))
                    .await;
                bail!("Error reading frame. {err}");
            }
        };
        match frame {
            // Reached EOF
            None => return Ok(()),
            Some(ClientFrame::IAmCamera { road, mile, limit }) => {
                let road_channel = map.get_or_create_road(road, limit);
                connection
                    .set_client_type(ClientType::Camera { mile, road_channel })
                    .await?;
            }
            Some(ClientFrame::IAmDispatcher { roads, .. }) => {
                connection
                    .set_client_type(ClientType::Dispatcher {
                        roads: roads.clone(),
                    })
                    .await?;
                let (tx, rx) = oneshot::channel();
                map.ticket_processor
                    .send(ProcessorCommand::NewDispatcher { roads, ch: tx })
                    .await
                    .unwrap();
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
                            write_channel.send(ticket_frame).await.unwrap();
                            // FIXME
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
                    connection
                        .error("Received plate, but client did not registered as a camera before")
                        .await;
                    bail!("Received plate, but client did not registered as a camera before");
                }
            },
            Some(ClientFrame::WantHeartbeat { interval }) => {
                if connection.heartbeat_set {
                    connection.error("Heartbeat already set").await;
                } else {
                    connection.heartbeat_set = true;
                    tokio::spawn(create_heartbeat(interval, connection.write_channel.clone()));
                };
            }
        };
    }
}

enum ClientType {
    Camera {
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
    client_type: Option<ClientType>,
    heartbeat_set: bool,
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
            client_type: None,
            heartbeat_set: false,
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
    }

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
