//! Chunked-NDJSON streaming for `POST /sessions/<id>/runs/stream`.
//!
//! Each `RunEvent` emitted by the agent loop lands as one HTTP
//! chunk in real time — no audit-table polling. Implementation
//! pairs an `mpsc::UnboundedChannel` with `tokio::select!`: the
//! task running `run_turn_with_events` sends events to the
//! channel; the handler reads them and writes chunks. When the
//! task completes (success or error), a final `{"event":"done"|
//! "error", ...}` chunk is written and the chunked-encoded body
//! terminated with `0\r\n\r\n`.
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

/// Cheap heuristic the listener uses to route streaming requests
/// before reading their bodies. Matches the unique URL path so
/// non-streaming `POST /sessions/<id>/runs` keeps its existing
/// one-shot dispatch.
pub(super) fn is_streaming_runs_request(req: &str) -> bool {
    if !req.starts_with("POST /sessions/") {
        return false;
    }
    let Some(rest) = req.strip_prefix("POST /sessions/") else {
        return false;
    };
    let Some((_, tail)) = rest.split_once('/') else {
        return false;
    };
    tail.starts_with("runs/stream ")
}

/// Run the agent loop and stream its `RunEvent` emissions back as
/// chunked NDJSON. Real-time — events land as soon as the agent
/// loop emits them (no polling).
pub(super) async fn serve_streamed_run(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let Some(store) = store else {
        return write_one_shot_error(out, "503 Service Unavailable", "session store unavailable")
            .await;
    };
    let Some(sid) = parse_session_id(req) else {
        return write_one_shot_error(out, "400 Bad Request", "invalid session id").await;
    };
    let body = body_from_request(req);
    let parsed: serde_json::Value = if body.is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                return write_one_shot_error(
                    out,
                    "400 Bad Request",
                    &format!("parse body: {e}"),
                )
                .await;
            }
        }
    };
    let user = parsed
        .get("user")
        .and_then(|v| v.as_str())
        .map(String::from);
    let max_rounds = parsed
        .get("max_tool_rounds")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(crate::handlers::chat::MAX_TOOL_ROUNDS);

    let head = "HTTP/1.1 200 OK\r\n\
                Content-Type: application/x-ndjson\r\n\
                Transfer-Encoding: chunked\r\n\
                \r\n";
    out.write_all(head.as_bytes()).await?;
    out.flush().await?;

    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            write_chunk(
                out,
                &format!("{{\"event\":\"error\",\"message\":\"cwd: {e}\"}}\n"),
            )
            .await?;
            return write_terminator(out).await;
        }
    };

    let store_clone = Arc::clone(store);
    let (tx, mut rx) = mpsc::unbounded_channel::<serde_json::Value>();
    let task = tokio::spawn(async move {
        crate::handlers::chat::run_turn_with_events(
            &store_clone,
            &cwd,
            sid,
            user.as_deref(),
            false,
            max_rounds,
            Some(&tx),
        )
        .await
    });

    // Drain events until the task finishes. tokio::select! lets us
    // forward each event chunk as soon as it arrives.
    loop {
        tokio::select! {
            event = rx.recv() => {
                let Some(payload) = event else {
                    break;
                };
                let line = match serde_json::to_string(&payload) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                write_chunk(out, &format!("{line}\n")).await?;
            }
        }
    }

    let summary = match task.await {
        Ok(Ok(result)) => serde_json::json!({
            "event": "done",
            "assistant_id": result.assistant_id.to_string(),
            "tokens_in": result.tokens_in,
            "tokens_out": result.tokens_out,
        }),
        Ok(Err(e)) => serde_json::json!({"event":"error","message": e.to_string()}),
        Err(e) => serde_json::json!({"event":"error","message": format!("task: {e}")}),
    };
    write_chunk(out, &format!("{summary}\n")).await?;
    write_terminator(out).await
}

/// Write one HTTP chunk: `<hex_size>\r\n<body>\r\n`.
async fn write_chunk(out: &mut TcpStream, body: &str) -> tokio::io::Result<()> {
    let hdr = format!("{:X}\r\n", body.len());
    out.write_all(hdr.as_bytes()).await?;
    out.write_all(body.as_bytes()).await?;
    out.write_all(b"\r\n").await?;
    out.flush().await?;
    Ok(())
}

/// Write the terminating zero-length chunk per RFC 7230.
async fn write_terminator(out: &mut TcpStream) -> tokio::io::Result<()> {
    out.write_all(b"0\r\n\r\n").await?;
    out.flush().await
}

async fn write_one_shot_error(
    out: &mut TcpStream,
    status: &str,
    msg: &str,
) -> tokio::io::Result<()> {
    let body = format!("{{\"error\":\"{msg}\"}}");
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    out.write_all(resp.as_bytes()).await
}

fn body_from_request(req: &str) -> &str {
    req.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

fn parse_session_id(req: &str) -> Option<SessionId> {
    let rest = req.strip_prefix("POST /sessions/")?;
    let (sid, _) = rest.split_once('/')?;
    Some(SessionId(uuid::Uuid::parse_str(sid).ok()?))
}
