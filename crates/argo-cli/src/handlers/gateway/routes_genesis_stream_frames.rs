//! SSE frame writers for `routes_genesis_stream`. Mechanical extraction
//! (no behaviour change) to keep the stream handler under the per-file LOC
//! budget — these own the chunked-transfer HTTP framing for the
//! `text/event-stream` response written directly to the TCP socket.

use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub(super) async fn open_sse_stream(out: &mut TcpStream, cors_headers: &str) -> tokio::io::Result<()> {
    let head = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         X-Accel-Buffering: no\r\n\
         Transfer-Encoding: chunked\r\n\
         {cors_headers}\r\n"
    );
    out.write_all(head.as_bytes()).await?;
    out.flush().await
}

pub(super) async fn write_progress(
    out: &mut TcpStream,
    phase: &str,
    message: &str,
    percent: u8,
) -> tokio::io::Result<()> {
    let payload = json!({ "phase": phase, "message": message, "percent": percent });
    write_event(out, "progress", &payload).await?;
    out.flush().await
}

pub(super) async fn write_event(out: &mut TcpStream, name: &str, payload: &Value) -> tokio::io::Result<()> {
    let body = format!("event: {name}\ndata: {payload}\n\n");
    write_chunk(out, &body).await
}

pub(super) async fn write_error_event(
    out: &mut TcpStream,
    phase: &str,
    message: &str,
    recoverable: bool,
) -> tokio::io::Result<()> {
    let payload = json!({
        "phase": phase,
        "message": message,
        "recoverable": recoverable,
    });
    write_event(out, "error", &payload).await?;
    out.flush().await?;
    write_terminator(out).await?;
    out.shutdown().await
}

async fn write_chunk(out: &mut TcpStream, body: &str) -> tokio::io::Result<()> {
    let hdr = format!("{:X}\r\n", body.len());
    out.write_all(hdr.as_bytes()).await?;
    out.write_all(body.as_bytes()).await?;
    out.write_all(b"\r\n").await?;
    out.flush().await?;
    Ok(())
}

async fn write_terminator(out: &mut TcpStream) -> tokio::io::Result<()> {
    out.write_all(b"0\r\n\r\n").await?;
    out.flush().await
}

pub(super) async fn write_one_shot_error(
    out: &mut TcpStream,
    status: &str,
    code: &str,
    message: &str,
    cors_headers: &str,
) -> tokio::io::Result<()> {
    let body = json!({ "error": { "code": code, "message": message } }).to_string();
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{cors_headers}\r\n{body}",
        body.len()
    );
    out.write_all(resp.as_bytes()).await?;
    out.flush().await?;
    out.shutdown().await
}
