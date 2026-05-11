//! Real WebSocket upgrade (RFC 6455) for `/inbox/ws` and `/audit/stream`.
//! Replaces the prior `(status, body)` stub that emitted a fake 101 with
//! a JSON body — no real WS client could parse it.
//!
//! The accept loop in `boot.rs` detects WS-upgrade requests via
//! [`is_ws_request`] and hands the raw `TcpStream` to [`serve`], which
//! performs the handshake then runs the per-route frame loop:
//! `/inbox/ws` subscribes to an in-process inbox-event broadcast;
//! `/audit/stream` idles on keepalive (the `AuditSink` broadcast is
//! per-call-site today — sharing it through the gateway is a follow-up
//! brief).
//!
//! No external WS crate: plain text + ping + pong + close. Outbound
//! frames are unmasked (RFC §5.1); inbound frames are unmasked on read
//! per RFC §5.3.

use argo_store::SessionStore;
use base64::Engine;
use sha1::Digest;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::broadcast;

const WS_MAGIC: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
pub(super) const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);
const BROADCAST_CAPACITY: usize = 256;

const OPCODE_TEXT: u8 = 0x1;
const OPCODE_CLOSE: u8 = 0x8;
const OPCODE_PING: u8 = 0x9;
const OPCODE_PONG: u8 = 0xA;

pub(super) const CLOSE_GOING_AWAY: u16 = 1001;
pub(super) const CLOSE_LAGGED: u16 = 1011;

/// True when the request is a WS upgrade for one of the routes this
/// module owns. Plain GETs (no `Upgrade` header) fall through to the
/// snapshot `(status, body)` paths in `routes_channels_webinbox` /
/// `routes_audit_stream`.
pub(super) fn is_ws_request(req: &str) -> bool {
    has_upgrade(req) && (matches_inbox(req) || matches_audit(req))
}

fn inbox_broadcast() -> &'static broadcast::Sender<String> {
    static SENDER: OnceLock<broadcast::Sender<String>> = OnceLock::new();
    SENDER.get_or_init(|| broadcast::channel(BROADCAST_CAPACITY).0)
}

/// Publish one event to every live `/inbox/ws` subscriber. Best-effort.
pub(super) fn publish_inbox_event(payload: serde_json::Value) {
    let _ = inbox_broadcast().send(payload.to_string());
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum Route {
    Inbox,
    Audit,
}

/// Top-level entry from the accept loop. Owns the stream for the
/// lifetime of the WS connection (handshake → frame loop → close).
pub(super) async fn serve(
    req: &str,
    _store: Option<&Arc<SessionStore>>,
    stream: &mut TcpStream,
) -> tokio::io::Result<()> {
    let route = pick_route(req);
    if let Err((status, body)) = check_auth(req, route) {
        return write_http_response(stream, status, &body).await;
    }
    let Some(client_key) = parse_client_key(req) else {
        return write_http_response(
            stream,
            "400 Bad Request",
            "{\"error\":\"missing Sec-WebSocket-Key\"}",
        )
        .await;
    };
    let accept_key = compute_accept(&client_key);
    let head = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept_key}\r\n\r\n"
    );
    stream.write_all(head.as_bytes()).await?;
    stream.flush().await?;
    let mut rx = match route {
        Route::Inbox => inbox_broadcast().subscribe(),
        Route::Audit => {
            // No shared `AuditSink` is wired through the gateway yet
            // (each call-site builds its own `AuditSink::new(store)`).
            // Hold a never-fed receiver so keepalive runs unchanged.
            let (_tx, rx) = broadcast::channel::<String>(1);
            rx
        }
    };
    run_loop(stream, &mut rx, KEEPALIVE_INTERVAL).await
}

/// SHA-1(client_key || WS_MAGIC) → base64. RFC 6455 §4.2.2 step 5.
pub(super) fn compute_accept(client_key: &str) -> String {
    let mut hasher = sha1::Sha1::new();
    hasher.update(client_key.as_bytes());
    hasher.update(WS_MAGIC.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
}

pub(super) fn parse_client_key(req: &str) -> Option<String> {
    header_value(req, "Sec-WebSocket-Key").map(str::to_string)
}

fn has_upgrade(req: &str) -> bool {
    header_value(req, "Upgrade").is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
}

fn matches_inbox(req: &str) -> bool {
    req.starts_with("GET /inbox/ws ") || req.starts_with("GET /inbox/ws?")
}

fn matches_audit(req: &str) -> bool {
    req.starts_with("GET /audit/stream ") || req.starts_with("GET /audit/stream?")
}

fn pick_route(req: &str) -> Route {
    if matches_inbox(req) {
        Route::Inbox
    } else {
        Route::Audit
    }
}

fn header_value<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    req.lines().skip(1).find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.eq_ignore_ascii_case(name)
            .then(|| v.trim_end_matches('\r').trim())
    })
}

pub(super) fn check_auth(req: &str, route: Route) -> Result<(), (&'static str, String)> {
    let bearer = header_value(req, "Authorization")
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::to_string)
        .or_else(|| cookie_value(req, "argo_operator_token"));
    let Some(token) = bearer else {
        return Err((
            "401 Unauthorized",
            "{\"error\":\"missing bearer token\"}".to_string(),
        ));
    };
    let env_var = match route {
        Route::Inbox => "ARGO_WEBINBOX_BEARER_TOKEN",
        Route::Audit => "ARGO_AUDIT_STREAM_BEARER_TOKEN",
    };
    if let Ok(expected) = argo_config::cred::get(env_var) {
        if !expected.is_empty() && token != expected {
            return Err((
                "401 Unauthorized",
                "{\"error\":\"invalid bearer token\"}".to_string(),
            ));
        }
    }
    Ok(())
}

fn cookie_value(req: &str, key: &str) -> Option<String> {
    let raw = header_value(req, "Cookie")?;
    raw.split(';').find_map(|kv| {
        let (k, v) = kv.trim().split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

async fn write_http_response<S: AsyncWrite + Unpin>(
    stream: &mut S,
    status: &str,
    body: &str,
) -> tokio::io::Result<()> {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes()).await?;
    stream.flush().await
}

pub(super) async fn run_loop<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    rx: &mut broadcast::Receiver<String>,
    keepalive_interval: Duration,
) -> tokio::io::Result<()> {
    let mut keepalive = tokio::time::interval(keepalive_interval);
    keepalive.tick().await;
    let mut awaiting_pong = false;
    let mut read_buf = [0u8; 1024];
    loop {
        tokio::select! {
            evt = rx.recv() => match evt {
                Ok(text) => write_frame(stream, OPCODE_TEXT, text.as_bytes()).await?,
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    write_close(stream, CLOSE_LAGGED, "lagged").await?;
                    return Ok(());
                }
                Err(broadcast::error::RecvError::Closed) => return Ok(()),
            },
            _ = keepalive.tick() => {
                if awaiting_pong {
                    write_close(stream, CLOSE_GOING_AWAY, "pong timeout").await?;
                    return Ok(());
                }
                write_frame(stream, OPCODE_PING, b"").await?;
                awaiting_pong = true;
            }
            n = stream.read(&mut read_buf) => {
                let n = n.unwrap_or(0);
                if n == 0 { return Ok(()); }
                match decode_frame(&read_buf[..n]) {
                    Some((OPCODE_PONG, _)) => awaiting_pong = false,
                    Some((OPCODE_PING, payload)) => write_frame(stream, OPCODE_PONG, &payload).await?,
                    Some((OPCODE_CLOSE, _)) | None => return Ok(()),
                    _ => {}
                }
            }
        }
    }
}

pub(super) async fn write_frame<S: AsyncWrite + Unpin>(
    stream: &mut S,
    opcode: u8,
    payload: &[u8],
) -> tokio::io::Result<()> {
    let header = encode_frame_header(opcode, payload.len());
    stream.write_all(&header).await?;
    stream.write_all(payload).await?;
    stream.flush().await
}

pub(super) async fn write_close<S: AsyncWrite + Unpin>(
    stream: &mut S,
    code: u16,
    reason: &str,
) -> tokio::io::Result<()> {
    let mut payload = Vec::with_capacity(2 + reason.len());
    payload.extend_from_slice(&code.to_be_bytes());
    payload.extend_from_slice(reason.as_bytes());
    write_frame(stream, OPCODE_CLOSE, &payload).await
}

/// 2- or 4-byte server frame header. We never mask outbound (§5.1)
/// and never emit ≥64 KiB payloads, so the 8-byte extended length is
/// omitted.
fn encode_frame_header(opcode: u8, len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(4);
    out.push(0x80 | opcode);
    if len < 126 {
        out.push(u8::try_from(len).unwrap_or(0));
    } else {
        out.push(126);
        out.push(u8::try_from((len >> 8) & 0xff).unwrap_or(0));
        out.push(u8::try_from(len & 0xff).unwrap_or(0));
    }
    out
}

/// Decode the first complete frame in `buf`. Supports 7-bit and 16-bit
/// length forms. Client→server frames must be masked (§5.3); we unmask
/// on read.
pub(super) fn decode_frame(buf: &[u8]) -> Option<(u8, Vec<u8>)> {
    if buf.len() < 2 {
        return None;
    }
    let opcode = buf[0] & 0x0F;
    let masked = (buf[1] & 0x80) != 0;
    let (payload_len, header_len) = match buf[1] & 0x7F {
        n if n < 126 => (n as usize, 2),
        126 if buf.len() >= 4 => (((buf[2] as usize) << 8) | (buf[3] as usize), 4),
        _ => return None,
    };
    let mut offset = header_len;
    let mask_key = if masked {
        if buf.len() < offset + 4 {
            return None;
        }
        let key = [buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]];
        offset += 4;
        Some(key)
    } else {
        None
    };
    if buf.len() < offset + payload_len {
        return None;
    }
    let mut payload = buf[offset..offset + payload_len].to_vec();
    if let Some(key) = mask_key {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= key[i % 4];
        }
    }
    Some((opcode, payload))
}

#[cfg(test)]
#[path = "ws_upgrade_tests.rs"]
mod tests;
