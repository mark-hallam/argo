//! `argo gateway probe [live|ready]` — hit the K8s-style health
//! probe endpoints (#630) and return exit 0 on success, non-zero
//! on failure. Useful for shell scripts / CI gates / boot-up
//! health checks that want a binary signal without parsing
//! `argo doctor`.
//!
//! Default kind is `ready` (DB-aware). Pass `live` for the
//! minimal liveness probe.
//!
//! Implementation note: argo-cli has no HTTP-client dep, so we
//! drive the request via raw `std::net::TcpStream` — same shape
//! the integration-test helpers use. Keeps the probe surface a
//! thin shim with no extra build-time cost.
use argo_types::{ArgoError, ArgoResult};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub async fn run(kind: String, host: String, port: u16, json: bool) -> ArgoResult<()> {
    let path = match kind.as_str() {
        "live" => "/health/live",
        "ready" => "/health/ready",
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown probe kind '{other}': expected 'live' or 'ready'"
            )));
        }
    };
    let result = probe_one(&host, port, path);
    if json {
        let payload = match &result {
            Ok((status_line, body)) => serde_json::json!({
                "kind": kind,
                "ok": status_line.starts_with("HTTP/1.1 200"),
                "status_line": status_line,
                "body": serde_json::from_str::<serde_json::Value>(body)
                    .unwrap_or(serde_json::Value::String(body.clone())),
            }),
            Err(e) => serde_json::json!({
                "kind": kind,
                "ok": false,
                "error": format!("{e}"),
            }),
        };
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        match &result {
            Ok((status_line, body)) => {
                let head = status_line.lines().next().unwrap_or(status_line);
                println!("{kind}: {head}\n  body: {body}");
            }
            Err(e) => println!("{kind}: error\n  {e}"),
        }
    }
    match result {
        Ok((status_line, _)) if status_line.starts_with("HTTP/1.1 200") => Ok(()),
        Ok((status_line, _)) => Err(ArgoError::Provider(format!("probe failed: {status_line}"))),
        Err(e) => Err(e),
    }
}

/// One-shot raw HTTP GET. Returns `(status_line, body)` on a
/// well-formed reply.
fn probe_one(host: &str, port: u16, path: &str) -> ArgoResult<(String, String)> {
    let mut stream = TcpStream::connect((host, port))
        .map_err(|e| ArgoError::Provider(format!("connect {host}:{port}: {e}")))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .ok();
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .map_err(|e| ArgoError::Provider(format!("read: {e}")))?;
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .ok_or_else(|| ArgoError::Provider(format!("malformed response: {raw}")))?;
    let status_line = head.lines().next().unwrap_or("").to_string();
    Ok((status_line, body.to_string()))
}
