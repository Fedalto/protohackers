use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use budget_chat::server::Server;

async fn start_server() -> SocketAddr {
    let server = Server::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap();

    tokio::spawn(async move { server.run().await });

    addr
}

#[tokio::test]
async fn test_1_client_connected() {
    let server = start_server().await;
    let mut client = TcpStream::connect(server).await.unwrap();

    let mut buffer = [0; 45];
    client.read_exact(&mut buffer).await.unwrap();
    let greetings = String::from_utf8(buffer.to_vec()).unwrap();
    assert_eq!(greetings, "Welcome to budgetchat! What shall I call you?");

    client.write_all("Leo".as_bytes()).await.unwrap();
}
