use super::build_http_client;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn heartbeat_chunks_keep_body_read_alive_past_base_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        read_request_headers(&mut socket).await;
        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: text/event-stream\r\n",
            "Transfer-Encoding: chunked\r\n",
            "\r\n"
        );
        socket.write_all(response.as_bytes()).await.unwrap();

        for _ in 0..5 {
            tokio::time::sleep(Duration::from_millis(35)).await;
            write_chunk(&mut socket, b": heartbeat\n\n").await;
        }

        tokio::time::sleep(Duration::from_millis(35)).await;
        write_chunk(&mut socket, b"data: done\n\n").await;
        socket.write_all(b"0\r\n\r\n").await.unwrap();
        socket.shutdown().await.unwrap();
    });

    let client = build_http_client(500, 75).unwrap();
    let start = Instant::now();
    let body = client
        .get(format!("http://{addr}/stream"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert!(
        start.elapsed() >= Duration::from_millis(200),
        "test must run longer than the configured read timeout"
    );
    assert!(body.contains(": heartbeat"));
    assert!(body.contains("data: done"));

    server.await.unwrap();
}

async fn read_request_headers(socket: &mut tokio::net::TcpStream) {
    let mut buf = [0_u8; 1024];
    let mut seen = Vec::new();
    loop {
        let n = socket.read(&mut buf).await.unwrap();
        assert!(n > 0, "client closed before sending request headers");
        seen.extend_from_slice(&buf[..n]);
        if seen.windows(4).any(|window| window == b"\r\n\r\n") {
            return;
        }
    }
}

async fn write_chunk(socket: &mut tokio::net::TcpStream, bytes: &[u8]) {
    socket
        .write_all(format!("{:x}\r\n", bytes.len()).as_bytes())
        .await
        .unwrap();
    socket.write_all(bytes).await.unwrap();
    socket.write_all(b"\r\n").await.unwrap();
    socket.flush().await.unwrap();
}
