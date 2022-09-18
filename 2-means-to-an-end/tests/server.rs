use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use tokio::net::{TcpListener, TcpStream};

use means_to_an_end::server;

async fn start_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move { server::run(listener).await });

    addr
}

#[tokio::test]
async fn test_server() {
    let server = start_server().await;
    let mut connection = TcpStream::connect(server).await.unwrap();

    let input = [
        0x49, 0x00, 0x00, 0x30, 0x39, 0x00, 0x00, 0x00, 0x65, // I 12345 101
        0x49, 0x00, 0x00, 0x30, 0x3a, 0x00, 0x00, 0x00, 0x66, // I 12346 102
        0x49, 0x00, 0x00, 0x30, 0x3b, 0x00, 0x00, 0x00, 0x64, // I 12347 100
        0x49, 0x00, 0x00, 0xa0, 0x00, 0x00, 0x00, 0x00, 0x05, // I 40960 5
        0x51, 0x00, 0x00, 0x30, 0x00, 0x00, 0x00, 0x40, 0x00, // Q 12288 16384
    ];
    connection.write_all(&input).await.unwrap();

    let mut response = [0u8; 4];
    connection.read_exact(&mut response).await.unwrap();

    assert_eq!(response, [0x00, 0x00, 0x00, 0x65]);
}
