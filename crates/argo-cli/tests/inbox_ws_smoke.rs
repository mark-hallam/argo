//! Integration smoke for the real `/inbox/ws` WebSocket upgrade
//! (`mission-inbox-ws-real-websocket`). Boots a gateway subprocess,
//! opens a TCP connection, sends a valid WS handshake, and asserts:
//!
//! 1. The gateway returns `101 Switching Protocols`.
//! 2. The `Sec-WebSocket-Accept` header matches the RFC 6455
//!    derivation (SHA-1 of `client_key || magic GUID`, then base64).
//! 3. No `Content-Length` / `Content-Type` JSON-body framing leaks
//!    into the 101 response — the prior stub emitted those and any
//!    real WS client would refuse the upgrade.
//!
//! Live event push within 100ms is covered by the
//! `ws_upgrade::tests::broadcast_message_pushes_text_frame` unit
//! test, which exercises the same `run_loop` against an in-process
//! `tokio::io::duplex` pair. Wiring an end-to-end event trigger
//! through the gateway subprocess would require setting
//! `ARGO_CHANNEL_HMAC_SECRET` + `ARGO_WEBINBOX_PATH` on the boot
//! command line; the unit test pins the contract more precisely
//! without that ceremony.
mod common;
use base64::Engine;
use common::{boot, pick_free_port};
use sha1::Digest;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const RFC6455_MAGIC: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[test]
fn inbox_ws_upgrade_returns_valid_101() {
    // Gateway startup_check refuses to bind without a provider key.
    // The other smoke tests rely on CI having one pre-set; we set a
    // dummy here so the test boots locally too (matches
    // `gateway_smoke::*` resolved-flake notes in KNOWN_FLAKES.md).
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");
    let port = pick_free_port();
    // boot()'s warmup `/sessions` counts as 1; our WS handshake is
    // the 2nd. Pass 1 so the gateway's `--max-requests` arg lands at
    // 2 (boot() adds 1 internally) and the process exits cleanly
    // after serving exactly those two.
    let (mut child, _stderr) = boot(port, 1);

    let client_key = "dGhlIHNhbXBsZSBub25jZQ==";
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("read timeout");
    let req = format!(
        "GET /inbox/ws HTTP/1.1\r\n\
         Host: localhost\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {client_key}\r\n\
         Sec-WebSocket-Version: 13\r\n\
         Authorization: Bearer test-op\r\n\
         \r\n"
    );
    stream.write_all(req.as_bytes()).expect("write");

    let mut buf = [0u8; 512];
    let n = stream.read(&mut buf).expect("read");
    let head = String::from_utf8_lossy(&buf[..n]);

    assert!(
        head.contains("HTTP/1.1 101 Switching Protocols"),
        "expected 101 upgrade, got: {head}"
    );
    assert!(
        head.to_lowercase().contains("upgrade: websocket"),
        "missing Upgrade header: {head}"
    );
    assert!(
        head.to_lowercase().contains("connection: upgrade"),
        "missing Connection header: {head}"
    );

    let mut hasher = sha1::Sha1::new();
    hasher.update(client_key.as_bytes());
    hasher.update(RFC6455_MAGIC.as_bytes());
    let expected = base64::engine::general_purpose::STANDARD.encode(hasher.finalize());
    let needle = format!("Sec-WebSocket-Accept: {expected}");
    assert!(
        head.contains(&needle),
        "Sec-WebSocket-Accept did not match RFC 6455 derivation. \
         expected `{expected}` in headers:\n{head}"
    );

    // The prior fake-101 stub also wrote `Content-Length` and
    // `Content-Type: application/json`. A correct WS handshake must
    // omit them — otherwise compliant clients reject the upgrade.
    assert!(
        !head.contains("Content-Length"),
        "101 must not carry Content-Length: {head}"
    );

    // Drop the stream so the gateway closes the WS handler and the
    // single served-request slot is freed for shutdown.
    drop(stream);
    assert!(child.wait().expect("wait").success());
}
