use super::wait_for;
use argo_mcp::McpContent;
use futures::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone)]
struct FixtureEvent {
    delay_ms: u64,
    frame: serde_json::Value,
}

fn request(id: &str) -> FixtureEvent {
    FixtureEvent {
        delay_ms: 0,
        frame: serde_json::json!({
            "method": "Network.requestWillBeSent",
            "params": { "requestId": id }
        }),
    }
}

fn response_after(delay_ms: u64, id: &str) -> FixtureEvent {
    FixtureEvent {
        delay_ms,
        frame: serde_json::json!({
            "method": "Network.responseReceived",
            "params": { "requestId": id }
        }),
    }
}

async fn spawn_fixture(events: Vec<FixtureEvent>) -> u16 {
    let ws_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ws_addr = ws_listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (stream, _) = ws_listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        let _ = ws.next().await.unwrap().unwrap();
        ws.send(Message::Text(
            serde_json::json!({"id": 1, "result": {}}).to_string(),
        ))
        .await
        .unwrap();
        for event in events {
            tokio::time::sleep(Duration::from_millis(event.delay_ms)).await;
            ws.send(Message::Text(event.frame.to_string()))
                .await
                .unwrap();
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    let http_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let http_addr = http_listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = http_listener.accept().await.unwrap();
        let mut buf = [0_u8; 1024];
        let _ = stream.read(&mut buf).await.unwrap();
        let body = serde_json::json!([{
            "type": "page",
            "webSocketDebuggerUrl": format!("ws://{ws_addr}/devtools/page/1")
        }])
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });
    http_addr.port()
}

fn text(result: &argo_mcp::CallToolResult) -> &str {
    match &result.content[0] {
        McpContent::Text { text } => text,
        _ => panic!("expected text"),
    }
}

#[tokio::test]
async fn network_idle_zero_fetches_resolves_after_quiet_period() {
    let port = spawn_fixture(Vec::new()).await;
    let result = wait_for(&serde_json::json!({
        "port": port,
        "mode": "network_idle",
        "quiet_period_ms": 20,
        "timeout_ms": 200
    }))
    .await;
    assert!(!result.is_error, "{}", text(&result));
    assert!(text(&result).contains("network idle after"));
}

#[tokio::test]
async fn network_idle_one_fetch_resolves_after_response_and_quiet_period() {
    let port = spawn_fixture(vec![request("a"), response_after(40, "a")]).await;
    let result = wait_for(&serde_json::json!({
        "port": port,
        "mode": "network_idle",
        "quiet_period_ms": 30,
        "timeout_ms": 300
    }))
    .await;
    assert!(!result.is_error, "{}", text(&result));
    assert!(text(&result).contains("quiet_period_ms=30"));
}

#[tokio::test]
async fn network_idle_continuous_fetches_timeout_at_timeout_ms() {
    let port = spawn_fixture(vec![
        request("a"),
        response_after(20, "a"),
        request("b"),
        response_after(20, "b"),
        request("c"),
        response_after(20, "c"),
    ])
    .await;
    let result = wait_for(&serde_json::json!({
        "port": port,
        "mode": "network_idle",
        "quiet_period_ms": 40,
        "timeout_ms": 90
    }))
    .await;
    assert!(result.is_error, "{}", text(&result));
    assert!(text(&result).contains("timed out after 90ms"));
}
