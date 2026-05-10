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
/// before reading their bodies. Matches every streaming URL path
/// so the one-shot `POST /sessions/<id>/runs` etc. keep their
/// existing one-shot dispatch.
pub(super) fn is_streaming_runs_request(req: &str) -> bool {
    streaming_kind(req).is_some()
}

/// Internal: which streaming endpoint matches, or None.
fn streaming_kind(req: &str) -> Option<StreamKind> {
    if req.starts_with("POST /forge/optimize/stream ") {
        return Some(StreamKind::ForgeOptimize);
    }
    let rest = req.strip_prefix("POST /sessions/")?;
    let (_, tail) = rest.split_once('/')?;
    if tail.starts_with("runs/stream ") {
        Some(StreamKind::Runs)
    } else if tail.starts_with("replay/stream ") {
        Some(StreamKind::Replay)
    } else {
        None
    }
}

#[derive(Copy, Clone)]
enum StreamKind {
    Runs,
    Replay,
    ForgeOptimize,
}

/// Run either the agent loop (`runs/stream`) or a session replay
/// (`replay/stream`) and stream its `RunEvent` emissions back as
/// chunked NDJSON. Real-time — events land as soon as the loop
/// emits them (no polling).
pub(super) async fn serve_streamed_run(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let Some(store) = store else {
        return write_one_shot_error(out, "503 Service Unavailable", "session store unavailable")
            .await;
    };
    // Forge optimize stream takes a different body shape (no
    // session id) — split it out before the session-flavoured
    // path picks up.
    if matches!(streaming_kind(req), Some(StreamKind::ForgeOptimize)) {
        return serve_streamed_forge_optimize(req, out).await;
    }
    let Some(sid_str) = parse_session_id_str(req) else {
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

    let kind = streaming_kind(req).unwrap_or(StreamKind::Runs);
    let (tx, mut rx) = mpsc::unbounded_channel::<serde_json::Value>();
    let task = match kind {
        StreamKind::Runs => {
            let user = parsed
                .get("user")
                .and_then(|v| v.as_str())
                .map(String::from);
            let max_rounds = parsed
                .get("max_tool_rounds")
                .and_then(serde_json::Value::as_u64)
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(crate::handlers::chat::MAX_TOOL_ROUNDS);
            let store_clone = Arc::clone(store);
            let cwd_clone = cwd.clone();
            let sid = match uuid::Uuid::parse_str(&sid_str) {
                Ok(u) => SessionId(u),
                Err(e) => {
                    write_chunk(
                        out,
                        &format!("{{\"event\":\"error\",\"message\":\"sid: {e}\"}}\n"),
                    )
                    .await?;
                    return write_terminator(out).await;
                }
            };
            TaskHandle::Run(tokio::spawn(async move {
                crate::handlers::chat::run_turn_with_events(
                    &store_clone,
                    &cwd_clone,
                    sid,
                    user.as_deref(),
                    false,
                    max_rounds,
                    Some(&tx),
                )
                .await
                .map(|r| {
                    serde_json::json!({
                        "event": "done",
                        "assistant_id": r.assistant_id.to_string(),
                        "tokens_in": r.tokens_in,
                        "tokens_out": r.tokens_out,
                    })
                })
            }))
        }
        StreamKind::Replay => {
            let model = parsed
                .get("model")
                .and_then(|v| v.as_str())
                .map(String::from);
            let provider = parsed
                .get("provider")
                .and_then(|v| v.as_str())
                .map(String::from);
            let sid_owned = sid_str.clone();
            TaskHandle::Replay(tokio::spawn(async move {
                crate::handlers::session_replay::replay_core_with_events(
                    &sid_owned,
                    model.as_deref(),
                    provider.as_deref(),
                    Some(&tx),
                )
                .await
                .map(|r| {
                    serde_json::json!({
                        "event": "done",
                        "source_id": r.source_id.0.to_string(),
                        "new_id": r.new_id.0.to_string(),
                        "user_message_count": r.user_message_count,
                    })
                })
            }))
        }
        // ForgeOptimize is short-circuited at the top of
        // `serve_streamed_run` and never reaches this match.
        StreamKind::ForgeOptimize => unreachable!(),
    };

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

    let summary = match task {
        TaskHandle::Run(h) => match h.await {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => serde_json::json!({"event":"error","message": e.to_string()}),
            Err(e) => serde_json::json!({"event":"error","message": format!("task: {e}")}),
        },
        TaskHandle::Replay(h) => match h.await {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => serde_json::json!({"event":"error","message": e.to_string()}),
            Err(e) => serde_json::json!({"event":"error","message": format!("task: {e}")}),
        },
    };
    write_chunk(out, &format!("{summary}\n")).await?;
    write_terminator(out).await
}

enum TaskHandle {
    Run(tokio::task::JoinHandle<argo_types::ArgoResult<serde_json::Value>>),
    Replay(tokio::task::JoinHandle<argo_types::ArgoResult<serde_json::Value>>),
}

/// Stream a forge-optimize run as chunked NDJSON. Body shape
/// is the same as the non-streaming `POST /forge/optimize`:
/// `{skill, examples, iterations?=12, budget_usd?=5.0}`. Unlike
/// the kick-off endpoint (which returns 202 immediately and the
/// caller polls `GET /forge/runs`), this one keeps the HTTP
/// connection open and writes one chunk per `start` / `iter` /
/// `done` / `budget_exceeded` event.
async fn serve_streamed_forge_optimize(req: &str, out: &mut TcpStream) -> tokio::io::Result<()> {
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return write_one_shot_error(out, "400 Bad Request", &format!("parse body: {e}"))
                .await;
        }
    };
    let skill = match parsed.get("skill").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return write_one_shot_error(out, "400 Bad Request", "missing or empty `skill`").await;
        }
    };
    let examples = match parsed.get("examples").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return write_one_shot_error(
                out,
                "400 Bad Request",
                "missing or empty `examples` (path to JSONL)",
            )
            .await;
        }
    };
    let iterations = parsed
        .get("iterations")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(12);
    let budget_usd = parsed
        .get("budget_usd")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(5.0);

    let head = "HTTP/1.1 200 OK\r\n\
                Content-Type: application/x-ndjson\r\n\
                Transfer-Encoding: chunked\r\n\
                \r\n";
    out.write_all(head.as_bytes()).await?;
    out.flush().await?;

    let (tx, mut rx) = mpsc::unbounded_channel::<serde_json::Value>();
    let task: tokio::task::JoinHandle<argo_types::ArgoResult<()>> = tokio::spawn(async move {
        crate::handlers::forge_optimize::run_with_events(
            &skill,
            &examples,
            iterations,
            budget_usd,
            true,
            Some(tx),
        )
        .await
    });
    while let Some(payload) = rx.recv().await {
        let line = match serde_json::to_string(&payload) {
            Ok(s) => s,
            Err(_) => continue,
        };
        write_chunk(out, &format!("{line}\n")).await?;
    }
    let summary = match task.await {
        Ok(Ok(())) => serde_json::json!({"event":"task_finished"}),
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

fn parse_session_id_str(req: &str) -> Option<String> {
    let rest = req.strip_prefix("POST /sessions/")?;
    let (sid, _) = rest.split_once('/')?;
    Some(sid.to_string())
}
