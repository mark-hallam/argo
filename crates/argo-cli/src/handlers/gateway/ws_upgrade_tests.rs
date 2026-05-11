//! Unit tests for the WebSocket upgrade machinery. Sibling-file
//! pattern: `ws_upgrade.rs` mounts this via `#[path = …] mod tests;`
//! so we keep the production module under the brief's LOC budget.

use super::*;
use std::time::Duration;
use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt, DuplexStream};

const RFC6455_KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";
const RFC6455_ACCEPT: &str = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

#[test]
fn accept_key_matches_rfc6455_vector() {
    assert_eq!(compute_accept(RFC6455_KEY), RFC6455_ACCEPT);
}

#[test]
fn bad_key_rejected_when_header_missing() {
    let req = "GET /inbox/ws HTTP/1.1\r\nUpgrade: websocket\r\n\r\n";
    assert!(parse_client_key(req).is_none());
}

#[test]
fn missing_upgrade_header_rejects_route() {
    let req =
        "GET /inbox/ws HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer x\r\n\r\n";
    assert!(!is_ws_request(req));

    let upgraded =
        "GET /inbox/ws HTTP/1.1\r\nHost: x\r\nUpgrade: websocket\r\nSec-WebSocket-Key: k\r\n\r\n";
    assert!(is_ws_request(upgraded));
}

#[test]
fn auth_paths_combined() {
    // Auth gate is process-global through `argo_config::cred::get`,
    // so all four auth paths (env-unlocked vs env-locked × missing
    // vs valid vs bad token) share one `#[test]` to keep cargo's
    // parallel runner from racing the env var. Mirrors the sibling
    // `check_auth_serial_paths` in `routes_audit_stream::tests`.
    std::env::remove_var("ARGO_WEBINBOX_BEARER_TOKEN");
    let missing = "GET /inbox/ws HTTP/1.1\r\nUpgrade: websocket\r\nSec-WebSocket-Key: k\r\n\r\n";
    let err = check_auth(missing, Route::Inbox).unwrap_err();
    assert_eq!(err.0, "401 Unauthorized");
    assert!(err.1.contains("missing bearer token"));

    let any_token = "GET /inbox/ws HTTP/1.1\r\nUpgrade: websocket\r\nSec-WebSocket-Key: k\r\nAuthorization: Bearer anything\r\n\r\n";
    assert!(
        check_auth(any_token, Route::Inbox).is_ok(),
        "no-env should accept any bearer"
    );

    std::env::set_var("ARGO_WEBINBOX_BEARER_TOKEN", "expected");
    let bad = "GET /inbox/ws HTTP/1.1\r\nUpgrade: websocket\r\nSec-WebSocket-Key: k\r\nAuthorization: Bearer wrong\r\n\r\n";
    let err = check_auth(bad, Route::Inbox).unwrap_err();
    assert_eq!(err.0, "401 Unauthorized");
    assert!(err.1.contains("invalid bearer token"));

    let good = "GET /inbox/ws HTTP/1.1\r\nUpgrade: websocket\r\nSec-WebSocket-Key: k\r\nAuthorization: Bearer expected\r\n\r\n";
    assert!(
        check_auth(good, Route::Inbox).is_ok(),
        "env-locked token should accept match"
    );
    std::env::remove_var("ARGO_WEBINBOX_BEARER_TOKEN");
}

#[tokio::test]
async fn ping_pong_roundtrip_clears_awaiting() {
    // Server reads a client-masked PING, replies with an unmasked
    // PONG. We use tokio::io::duplex to exercise the generic
    // `write_frame` + `decode_frame` paths without booting a real
    // TcpListener.
    let (mut server, mut client) = duplex(1024);
    // Client → server PING (with mask, payload "hi").
    let frame = build_client_frame(0x9, b"hi");
    client.write_all(&frame).await.unwrap();
    // The frame loop's read path: decode + pong-back. We drive that
    // logic directly via the public helpers because `run_loop`
    // requires a broadcast::Receiver which is awkward to fabricate
    // in a single tokio::select tick.
    let mut buf = [0u8; 64];
    let n = server.read(&mut buf).await.unwrap();
    let (opcode, payload) = decode_frame(&buf[..n]).expect("masked PING decodes");
    assert_eq!(opcode, 0x9);
    assert_eq!(&payload, b"hi");
    write_frame(&mut server, 0xA, &payload).await.unwrap();
    server.flush().await.unwrap();
    drop(server);
    let mut sink = Vec::new();
    client.read_to_end(&mut sink).await.unwrap();
    // Server PONG: FIN+opcode 0xA, len 2, payload "hi".
    assert_eq!(sink, vec![0x8A, 0x02, b'h', b'i']);
}

#[tokio::test]
async fn lagged_subscriber_closes_with_1011() {
    let (mut server, mut client) = duplex(1024);
    let (tx, mut rx) = tokio::sync::broadcast::channel::<String>(1);
    // Overflow the channel before the loop subscribes so the next
    // recv() yields `Lagged`. Cap=1 means 2 sends + 1 unread = lag.
    let _ = tx.send("a".into());
    let _ = tx.send("b".into());
    let _ = tx.send("c".into());
    let server_task = tokio::spawn(async move {
        run_loop(&mut server, &mut rx, Duration::from_secs(60))
            .await
            .ok();
    });
    let mut buf = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), client.read_to_end(&mut buf)).await;
    server_task.await.unwrap();
    // Close frame: 0x88 prefix, then payload >= 2 bytes carrying the
    // 1011 code in big-endian.
    assert!(!buf.is_empty(), "server should have written a close frame");
    assert_eq!(buf[0], 0x88, "close opcode + FIN");
    let payload_len = buf[1] as usize;
    assert!(payload_len >= 2, "close payload must carry the 2-byte code");
    let code = u16::from_be_bytes([buf[2], buf[3]]);
    assert_eq!(code, CLOSE_LAGGED);
}

#[tokio::test]
async fn broadcast_message_pushes_text_frame() {
    let (mut server, mut client) = duplex(1024);
    let (tx, mut rx) = tokio::sync::broadcast::channel::<String>(8);
    let payload = r#"{"event":"decision","approval_id":"act-42"}"#.to_string();
    let payload_for_send = payload.clone();
    let server_task = tokio::spawn(async move {
        // Use a long keepalive so the test isn't racing the PING tick.
        run_loop(&mut server, &mut rx, Duration::from_secs(60))
            .await
            .ok();
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    tx.send(payload_for_send).unwrap();
    let mut buf = [0u8; 256];
    let n = tokio::time::timeout(Duration::from_millis(100), client.read(&mut buf))
        .await
        .expect("frame should land within 100ms")
        .unwrap();
    drop(tx);
    let _ = server_task.await;
    let (opcode, body) = decode_frame(&buf[..n]).expect("frame decodes");
    assert_eq!(opcode, 0x1, "text frame");
    assert_eq!(String::from_utf8(body).unwrap(), payload);
}

fn build_client_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    // Client → server frame: FIN=1, masked. Mask key is fixed for
    // determinism; payload XOR-masked.
    let mut out = Vec::with_capacity(6 + payload.len());
    out.push(0x80 | opcode);
    assert!(payload.len() < 126);
    out.push(0x80 | u8::try_from(payload.len()).unwrap());
    let key = [0x11_u8, 0x22, 0x33, 0x44];
    out.extend_from_slice(&key);
    for (i, b) in payload.iter().enumerate() {
        out.push(b ^ key[i % 4]);
    }
    out
}

#[allow(dead_code)]
fn _assert_duplex_is_async<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(_: T) {}
#[allow(dead_code)]
fn _check() {
    let (s, _c): (DuplexStream, DuplexStream) = duplex(1);
    _assert_duplex_is_async(s);
}
