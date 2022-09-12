use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use tokio::net::{TcpListener, TcpStream};

use prime_time::server;

async fn start_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move { server::run(listener).await });

    addr
}

#[tokio::test]
async fn test_valid_json() {
    let server = start_server().await;
    let mut connection = TcpStream::connect(server).await.unwrap();

    let input = b"{\"method\":\"isPrime\",\"number\":1}\n\
                            {\"method\":\"isPrime\",\"number\":2}\n";
    connection.write_all(input).await.unwrap();

    let mut response = [0; 69];
    connection.read_exact(&mut response).await.unwrap();

    let expected_response = b"{\"method\":\"isPrime\",\"prime\":false}\n\
                                        {\"method\":\"isPrime\",\"prime\":true}\n";
    assert_eq!(&response, expected_response);
}

#[tokio::test]
async fn test_invalid_json() {
    let server = start_server().await;
    let mut connection = TcpStream::connect(server).await.unwrap();

    let input = b"{\n";
    connection.write_all(input).await.unwrap();

    let mut response = [0; 25];
    connection.read_exact(&mut response).await.unwrap();

    let expected_response = b"{\"error\":\"invalid json\"}\n";
    assert_eq!(
        String::from_utf8(expected_response.to_vec()).unwrap(),
        String::from_utf8(response.to_vec()).unwrap(),
    );
}

#[tokio::test]
async fn test_invalid_method() {
    let server = start_server().await;
    let mut connection = TcpStream::connect(server).await.unwrap();

    let input = b"{\"method\":\"notPrime\",\"number\":1}\n";
    connection.write_all(input).await.unwrap();

    let mut response = [0; 27];
    connection.read_exact(&mut response).await.unwrap();

    let expected_response = b"{\"error\":\"invalid method\"}\n";
    assert_eq!(
        String::from_utf8(expected_response.to_vec()).unwrap(),
        String::from_utf8(response.to_vec()).unwrap(),
    );
}
