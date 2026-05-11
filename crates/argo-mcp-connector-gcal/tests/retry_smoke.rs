//! Integration smoke: confirms the retry layer absorbs transient upstream
//! 5xxs and the gcal client returns the eventual 200 body, with no public
//! API change.
//!
//! Strategy: stand up a tokio TCP mock server on a free port, configure
//! `GcalConfig` to point at it, and reply 503 twice then 200 once.
use argo_mcp_connector_gcal::{GcalClient, GcalConfig};
use serde_json::json;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const BODY_503: &str = "{\"error\": \"upstream blip\"}";
const BODY_200: &str = "{\"items\": [], \"kind\": \"calendar#events\"}";

async fn drain_request(stream: &mut tokio::net::TcpStream) {
    let mut buf = [0u8; 4096];
    while let Ok(n) = stream.read(&mut buf).await {
        if n == 0 {
            break;
        }
        let chunk = &buf[..n];
        if chunk.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
}

async fn write_response(stream: &mut tokio::net::TcpStream, status_line: &str, body: &str) {
    let response = format!(
        "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{body}",
        len = body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
    let _ = stream.shutdown().await;
}

#[tokio::test]
async fn list_events_retries_503_503_then_200() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let counter = Arc::new(AtomicU32::new(0));
    let counter_srv = counter.clone();

    let server = tokio::spawn(async move {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().await.expect("accept");
            drain_request(&mut stream).await;
            let n = counter_srv.fetch_add(1, Ordering::SeqCst) + 1;
            if n < 3 {
                write_response(&mut stream, "HTTP/1.1 503 Service Unavailable", BODY_503).await;
            } else {
                write_response(&mut stream, "HTTP/1.1 200 OK", BODY_200).await;
                break;
            }
        }
    });

    let client = GcalClient::new(GcalConfig {
        access_token: "test-token".into(),
        calendar_id: "primary".into(),
        base_url: format!("http://{addr}"),
    });
    let result = client.list_events(&json!({"max_results": 1})).await;
    server.await.expect("server task");

    let v = result.expect("retry layer must surface the eventual 200");
    assert_eq!(v["kind"], "calendar#events");
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}
