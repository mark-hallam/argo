//! 503 Service Unavailable middleware for planned downtime. Gated by
//! `ARGO_MAINTENANCE_MODE=1`; `/health[/...]` still answers so external
//! liveness probes don't trip. Optional `ARGO_MAINTENANCE_ETA` is
//! echoed in the JSON body so the SPA can render an operator ETA.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub(super) struct MaintenanceCfg {
    pub(super) eta: Option<String>,
}

pub(super) fn from_env() -> Option<MaintenanceCfg> {
    if std::env::var("ARGO_MAINTENANCE_MODE").ok().as_deref() == Some("1") {
        Some(MaintenanceCfg {
            eta: std::env::var("ARGO_MAINTENANCE_ETA").ok().filter(|s| !s.is_empty()),
        })
    } else {
        None
    }
}

/// If the gateway is in maintenance mode, write a `503` response and
/// return `true`. Liveness paths (`/health[/...]`) fall through. The
/// stream is peeked, then drained when 503ing so the caller skips its
/// own read.
pub(super) async fn try_serve(stream: &mut TcpStream, cfg: Option<&MaintenanceCfg>) -> bool {
    let Some(cfg) = cfg else {
        return false;
    };
    let mut peek_buf = [0_u8; 1024];
    let n = stream.peek(&mut peek_buf).await.unwrap_or(0);
    let req = std::str::from_utf8(&peek_buf[..n]).unwrap_or("");
    let path = req.split_whitespace().nth(1).unwrap_or("");
    if path == "/health" || path.starts_with("/health/") || path.starts_with("/health?") {
        return false;
    }
    let mut drain = [0_u8; 4096];
    let _ = stream.read(&mut drain).await;
    let eta = cfg.eta.as_deref().unwrap_or("");
    let body = format!("{{\"error\":\"maintenance\",\"retry_after\":\"{eta}\"}}");
    let resp = format!(
        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nRetry-After: 3600\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body,
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.shutdown().await;
    true
}

#[cfg(test)]
mod tests {
    fn is_health(path: &str) -> bool {
        path == "/health" || path.starts_with("/health/") || path.starts_with("/health?")
    }

    #[test]
    fn health_paths_are_excluded_so_liveness_probes_still_answer() {
        assert!(is_health("/health"));
        assert!(is_health("/health/store"));
        assert!(is_health("/health?probe=1"));
        assert!(!is_health("/healthz"));
        assert!(!is_health("/api/v1/approvals"));
    }
}
