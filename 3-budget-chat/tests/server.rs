use std::net::SocketAddr;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use budget_chat::server::Server;

async fn start_server() -> SocketAddr {
    let server = Server::new("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap();

    tokio::spawn(async move { server.run().await });

    addr
}

#[tokio::test]
async fn test_2_clients_connected() {
    let server = start_server().await;
    let connection1 = TcpStream::connect(server).await.unwrap();
    let mut connection1 = BufReader::new(connection1);

    let mut buffer = String::new();
    connection1.read_line(&mut buffer).await.unwrap();
    assert_eq!(buffer, "Welcome to budgetchat! What shall I call you?\n");

    connection1.write_all("Leo\n".as_bytes()).await.unwrap();
    buffer.clear();
    connection1.read_line(&mut buffer).await.unwrap();
    assert_eq!(buffer, "* Chatting now: \n");

    let connection2 = TcpStream::connect(server).await.unwrap();
    let mut connection2 = BufReader::new(connection2);

    buffer.clear();
    connection2.read_line(&mut buffer).await.unwrap();
    assert_eq!(buffer, "Welcome to budgetchat! What shall I call you?\n");

    connection2.write_all("Ana\n".as_bytes()).await.unwrap();
    buffer.clear();
    connection2.read_line(&mut buffer).await.unwrap();
    assert_eq!(buffer, "* Chatting now: Leo\n");

    buffer.clear();
    connection1.read_line(&mut buffer).await.unwrap();
    assert_eq!(buffer, "* Ana has entered the room\n");

    connection1.write_all("Hi!\n".as_bytes()).await.unwrap();
    buffer.clear();
    connection2.read_line(&mut buffer).await.unwrap();
    assert_eq!(buffer, "[Leo] Hi!\n");
}
