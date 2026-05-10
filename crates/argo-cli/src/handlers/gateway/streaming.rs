//! Chunked streaming for large gateway list GETs (sessions list,
//! message backfill, audit-recent). Bridges sqlx's row-at-a-time
//! `fetch()` Stream into chunked HTTP so memory stays flat and TTFB
//! is one-row latency. Wire formats: `application/json` (JSON array,
//! byte-identical to buffered path) and `application/x-ndjson` (one
//! row per line, opt-in via Accept). The listener-loop dispatcher
//! only fires when the path matches a streaming-eligible route AND
//! content-negotiation opts in (Accept, `?stream=true`, `?limit>1k`)
//! — all other traffic falls through to the buffered route table.
use argo_store::SessionStore;
use argo_types::ArgoResult;
use futures::{Stream, StreamExt};
use serde::Serialize;
use std::sync::Arc;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

/// True when the request is one of the three streaming-eligible
/// list GETs AND the caller has opted in to streaming (Accept
/// header, `?stream=true`, or `?limit>1000`). Used by the
/// listener-loop dispatcher to pick the streaming branch.
pub(super) fn is_streaming_list_request(req: &str) -> bool {
    streaming_list_kind(req).is_some() && streaming_negotiated(req)
}

#[derive(Copy, Clone, Debug)]
pub(super) enum ListKind {
    Sessions,
    Messages,
    AuditRecent,
}

/// Which streaming route matches, or `None` for non-streaming
/// paths. Lives next to `is_streaming_list_request` so callers
/// can dispatch off the same path table.
pub(super) fn streaming_list_kind(req: &str) -> Option<ListKind> {
    if req.starts_with("GET /sessions ") || req.starts_with("GET /sessions?") {
        return Some(ListKind::Sessions);
    }
    if req.starts_with("GET /audit/recent ") || req.starts_with("GET /audit/recent?") {
        return Some(ListKind::AuditRecent);
    }
    let rest = req.strip_prefix("GET /sessions/")?;
    let (_sid, tail) = rest.split_once('/')?;
    if tail.starts_with("messages ") || tail.starts_with("messages?") {
        Some(ListKind::Messages)
    } else {
        None
    }
}

/// JSON array vs newline-delimited.
#[derive(Copy, Clone, Debug)]
pub(super) enum WireFmt {
    JsonArray,
    Ndjson,
}

fn streaming_negotiated(req: &str) -> bool {
    accept_ndjson(req) || query_flag_true(req, "stream") || query_limit_over(req, 1000)
}

pub(super) fn pick_wire_fmt(req: &str) -> WireFmt {
    if accept_ndjson(req) {
        WireFmt::Ndjson
    } else {
        WireFmt::JsonArray
    }
}

fn accept_ndjson(req: &str) -> bool {
    accept_header(req).is_some_and(|h| h.contains("application/x-ndjson"))
}

fn accept_header(req: &str) -> Option<&str> {
    req.lines().find_map(|l| {
        let l = l.trim_end_matches('\r');
        l.split_once(':').and_then(|(k, v)| {
            k.eq_ignore_ascii_case("accept").then(|| v.trim())
        })
    })
}

fn query_flag_true(req: &str, key: &str) -> bool {
    request_query(req)
        .and_then(|q| query_get(q, key))
        .as_deref()
        == Some("true")
}

fn query_limit_over(req: &str, threshold: u32) -> bool {
    request_query(req)
        .and_then(|q| query_get(q, "limit"))
        .and_then(|v| v.parse::<u32>().ok())
        .is_some_and(|n| n > threshold)
}

fn request_query(req: &str) -> Option<&str> {
    let line = req.lines().next()?;
    let path = line.split_whitespace().nth(1)?;
    path.split_once('?').map(|(_, q)| q)
}

fn query_get(q: &str, key: &str) -> Option<String> {
    for kv in q.split('&') {
        let (k, v) = kv.split_once('=')?;
        if k == key {
            return Some(v.to_string());
        }
    }
    None
}

/// Top-level dispatcher invoked by the listener loop. Applies the
/// same bearer-auth gate the buffered routes get (the streaming
/// branch runs upstream of `routes::dispatch_route`), then hands
/// off to the per-route streaming handler.
pub(super) async fn serve_streamed_list(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    if let Some((status, body)) = super::routes_auth::check_bearer_auth(req) {
        return write_buffered(out, status, &body).await;
    }
    let Some(store) = store else {
        return write_buffered(
            out,
            "503 Service Unavailable",
            "{\"error\":\"session store unavailable\"}",
        )
        .await;
    };
    let Some(kind) = streaming_list_kind(req) else {
        return write_buffered(out, "404 Not Found", "{\"error\":\"not found\"}").await;
    };
    let fmt = pick_wire_fmt(req);
    match kind {
        ListKind::Sessions => super::routes_top_sessions::stream_sessions(req, store, out, fmt).await,
        ListKind::Messages => super::routes_msgs::stream_session_messages(req, store, out, fmt).await,
        ListKind::AuditRecent => super::routes_audit::stream_audit_recent(req, store, out, fmt).await,
    }
}

/// Drain `stream` to `out` as one chunked-encoded JSON array.
/// Empty streams emit `[]\n`. Mid-stream errors kill the
/// connection (loud + obvious — chunked transfer cannot rewind).
pub(super) async fn stream_json_array<T, S, W>(
    out: &mut W,
    stream: S,
) -> tokio::io::Result<()>
where
    S: Stream<Item = ArgoResult<T>>,
    T: Serialize,
    W: AsyncWrite + Unpin,
{
    write_chunked_head(out, "application/json").await?;
    let mut first = true;
    let mut buf = String::with_capacity(64);
    futures::pin_mut!(stream);
    while let Some(item) = stream.next().await {
        let value = item.map_err(stream_io_err)?;
        let payload = serde_json::to_string(&value).map_err(stream_io_err)?;
        buf.clear();
        if first {
            buf.push('[');
            first = false;
        } else {
            buf.push(',');
        }
        buf.push('\n');
        buf.push_str(&payload);
        write_chunk(out, &buf).await?;
    }
    if first {
        write_chunk(out, "[]\n").await?;
    } else {
        write_chunk(out, "\n]\n").await?;
    }
    write_terminator(out).await
}

/// Drain `stream` to `out` as chunked NDJSON — one JSON value per
/// line, no array brackets. Empty streams emit zero data chunks
/// (still HTTP 200). Mid-stream errors kill the connection.
pub(super) async fn stream_ndjson<T, S, W>(
    out: &mut W,
    stream: S,
) -> tokio::io::Result<()>
where
    S: Stream<Item = ArgoResult<T>>,
    T: Serialize,
    W: AsyncWrite + Unpin,
{
    write_chunked_head(out, "application/x-ndjson").await?;
    let mut buf = String::with_capacity(64);
    futures::pin_mut!(stream);
    while let Some(item) = stream.next().await {
        let value = item.map_err(stream_io_err)?;
        let line = serde_json::to_string(&value).map_err(stream_io_err)?;
        buf.clear();
        buf.push_str(&line);
        buf.push('\n');
        write_chunk(out, &buf).await?;
    }
    write_terminator(out).await
}

async fn write_chunked_head<W: AsyncWrite + Unpin>(
    out: &mut W,
    content_type: &str,
) -> tokio::io::Result<()> {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nTransfer-Encoding: chunked\r\n\r\n"
    );
    out.write_all(head.as_bytes()).await?;
    out.flush().await
}

async fn write_chunk<W: AsyncWrite + Unpin>(
    out: &mut W,
    body: &str,
) -> tokio::io::Result<()> {
    let hdr = format!("{:X}\r\n", body.len());
    out.write_all(hdr.as_bytes()).await?;
    out.write_all(body.as_bytes()).await?;
    out.write_all(b"\r\n").await?;
    out.flush().await
}

async fn write_terminator<W: AsyncWrite + Unpin>(out: &mut W) -> tokio::io::Result<()> {
    out.write_all(b"0\r\n\r\n").await?;
    out.flush().await
}

async fn write_buffered<W: AsyncWrite + Unpin>(
    out: &mut W,
    status: &str,
    body: &str,
) -> tokio::io::Result<()> {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    out.write_all(resp.as_bytes()).await?;
    out.flush().await
}

fn stream_io_err<E: std::fmt::Display>(e: E) -> tokio::io::Error {
    tokio::io::Error::other(e.to_string())
}

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
