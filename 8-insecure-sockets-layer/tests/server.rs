use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use insecure_sockets_layer::server::Server;

async fn start_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server = Server::new(listener);
    let address = server.local_addr();

    tokio::spawn(server.run());

    address
}

#[tokio::test]
async fn test_server() {
    let server = start_server().await;
    let mut connection = TcpStream::connect(server).await.unwrap();

    let input = [
        0x02, 0x7b, 0x05, 0x01, 0x00, // xor(123), add_pos, reverse_bits
        // 4x dog,5x car\n
        0xf2, 0x20, 0xba, 0x44, 0x18, 0x84, 0xba, 0xaa, 0xd0, 0x26, 0x44, 0xa4, 0xa8, 0x7e,
    ];

    connection.write_all(&input).await.unwrap();

    let mut response = [0u8; 7];
    connection.read_exact(&mut response).await.unwrap();
    let expected_response = [0x72, 0x20, 0xba, 0xd8, 0x78, 0x70, 0xee]; // 5x car\n
    assert_eq!(response, expected_response);

    let input = [
        0x6a, 0x48, 0xd6, 0x58, 0x34, 0x44, 0xd6, 0x7a, 0x98, 0x4e, 0x0c, 0xcc, 0x94, 0x31,
    ]; // 3x rat,2x cat\n
    connection.write_all(&input).await.unwrap();

    let mut response = [0u8; 7];
    connection.read_exact(&mut response).await.unwrap();
    let expected_response = [0xf2, 0xd0, 0x26, 0xc8, 0xa4, 0xd8, 0x7e]; // 3x rat\n
    assert_eq!(response, expected_response);
}
