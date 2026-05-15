use super::*;
use crate::manifest::PreferRule;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn discovery_spec(refresh_interval_hours: u32) -> DiscoverySpec {
    DiscoverySpec {
        endpoint: "{base_url}/v1/models".into(),
        include_pattern: "^gpt-".into(),
        exclude_pattern: Some("preview|legacy|deprecated".into()),
        prefer: PreferRule::HighestMinor,
        refresh_interval_hours,
    }
}

#[test]
fn latest_cache_defaults_to_static_model_until_refreshed() {
    let cache = LatestModelCache::new(discovery_spec(24), "gpt-4o-mini");

    assert_eq!(cache.refresh_interval(), Duration::from_secs(24 * 60 * 60));
    assert_eq!(cache.cached_model(), None);
    assert_eq!(cache.resolve_request_model("latest"), "gpt-4o-mini");
    assert_eq!(cache.resolve_request_model("gpt-4.1"), "gpt-4.1");
}

#[test]
fn latest_cache_fetches_once_until_refresh_interval_expires() {
    let cache = LatestModelCache::new(discovery_spec(1), "gpt-4o-mini");
    let now = Instant::now();
    let mut calls = 0;

    let first = cache
        .refresh_if_due_with(now, || {
            calls += 1;
            Ok(vec!["gpt-4.0", "gpt-4.1", "gpt-4.5-preview"])
        })
        .unwrap();
    assert_eq!(first.as_deref(), Some("gpt-4.1"));
    assert_eq!(calls, 1);

    let before_due = now + Duration::from_secs(59 * 60);
    let second = cache
        .refresh_if_due_with(before_due, || {
            calls += 1;
            Ok(vec!["gpt-4.2"])
        })
        .unwrap();
    assert_eq!(second, None);
    assert_eq!(calls, 1);
    assert_eq!(cache.resolve_request_model("latest"), "gpt-4.1");

    let after_due = now + Duration::from_secs(61 * 60);
    let third = cache
        .refresh_if_due_with(after_due, || {
            calls += 1;
            Ok(vec!["gpt-4.0", "gpt-4.2"])
        })
        .unwrap();
    assert_eq!(third.as_deref(), Some("gpt-4.2"));
    assert_eq!(calls, 2);
}

#[test]
fn latest_cache_keeps_previous_model_when_fetch_fails() {
    let cache = LatestModelCache::new(discovery_spec(1), "gpt-4o-mini");
    let now = Instant::now();
    cache
        .refresh_if_due_with(now, || Ok(vec!["gpt-4.0", "gpt-4.1"]))
        .unwrap();

    let err = cache
        .refresh_if_due_with::<&str, _>(now + Duration::from_secs(61 * 60), || {
            Err(argo_types::ArgoError::Provider("mock endpoint down".into()))
        })
        .unwrap_err();
    assert!(format!("{err}").contains("mock endpoint down"));
    assert_eq!(cache.resolve_request_model("latest"), "gpt-4.1");
}

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
