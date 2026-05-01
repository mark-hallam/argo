//! `GET /metrics` — Prometheus-style text exposition of the
//! key counters / gauges the gateway can answer cheaply from
//! the local DB. Lives in its own module because (a) the body
//! is text-format (not JSON) so the response is special-cased
//! at the listener level instead of going through
//! `dispatch_route`, and (b) keeps `routes_meta.rs` under the
//! 500-LOC ceiling.
//!
//! Format: <https://prometheus.io/docs/instrumenting/exposition_formats/#text-based-format>.
//! Each metric is preceded by `# HELP …` + `# TYPE …`. We do
//! not implement the full OpenMetrics spec (no exemplars, no
//! histograms) — just the gauges + counters that map cleanly
//! onto the rows already in the store.
use argo_store::SessionStore;
use std::fmt::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

/// Total /metrics scrapes since the gateway booted. Bumped at
/// the top of `render`. Self-referencing — Prometheus' own
/// `up` metric is the canonical scrape signal, but exposing
/// the counter here lets ops detect runaway pollers.
static SCRAPE_COUNT: AtomicU64 = AtomicU64::new(0);

/// Gateway boot time. Initialised once at gateway startup
/// (see `mod.rs::start`) so `argo_uptime_seconds` reports
/// real time-since-process-start. Falls back to lazy
/// init-on-first-scrape if startup forgot to call
/// `init_boot_time`.
static BOOT_TIME: OnceLock<Instant> = OnceLock::new();

/// Stamp the gateway's boot time. Idempotent — only the first
/// caller wins. Called from `gateway::start` once the listener
/// is bound.
pub(super) fn init_boot_time() {
    let _ = BOOT_TIME.set(Instant::now());
}

/// Reset the in-process scrape counter to zero. Returns the
/// previous value. Used by `POST /metrics/reset` (test-harness +
/// ops escape hatch). Boot time is left alone — `argo_uptime_seconds`
/// still reports real time-since-start.
pub(super) fn reset_scrape_count() -> u64 {
    SCRAPE_COUNT.swap(0, Ordering::Relaxed)
}

/// Returns the rendered exposition body. Errors from any single
/// query are surfaced as `# error: …` comment lines + a `0`
/// fallback so a partial DB never breaks the whole scrape.
pub(super) async fn render(store: Option<&Arc<SessionStore>>) -> String {
    let scrape_n = SCRAPE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let mut out = String::with_capacity(1024);
    let _ = writeln!(
        out,
        "# HELP argo_build_info Build info as labels (constant 1)\n\
         # TYPE argo_build_info gauge\n\
         argo_build_info{{version=\"{}\"}} 1",
        env!("CARGO_PKG_VERSION")
    );
    let boot = BOOT_TIME.get_or_init(Instant::now);
    let _ = writeln!(
        out,
        "\n# HELP argo_uptime_seconds Seconds since the gateway started serving /metrics\n\
         # TYPE argo_uptime_seconds gauge\n\
         argo_uptime_seconds {}",
        boot.elapsed().as_secs()
    );
    let _ = writeln!(
        out,
        "\n# HELP argo_metrics_scrapes_total Total scrapes since gateway boot\n\
         # TYPE argo_metrics_scrapes_total counter\n\
         argo_metrics_scrapes_total {scrape_n}"
    );

    let Some(store) = store else {
        let _ = writeln!(
            out,
            "\n# HELP argo_store_available Whether the session store opened (1=yes, 0=no)\n\
             # TYPE argo_store_available gauge\n\
             argo_store_available 0"
        );
        return out;
    };
    let _ = writeln!(
        out,
        "\n# HELP argo_store_available Whether the session store opened (1=yes, 0=no)\n\
         # TYPE argo_store_available gauge\n\
         argo_store_available 1"
    );

    let pool = store.pool();
    push_count(&mut out, pool, "argo_sessions_total", "Recorded sessions",
        "SELECT COUNT(*) FROM session").await;
    push_count(&mut out, pool, "argo_sessions_open",
        "Sessions with no ended_at stamp (still open)",
        "SELECT COUNT(*) FROM session WHERE ended_at IS NULL").await;
    push_count(&mut out, pool, "argo_messages_total", "Recorded messages",
        "SELECT COUNT(*) FROM message").await;
    push_count(&mut out, pool, "argo_audit_rows_total", "Audit-chain rows",
        "SELECT COUNT(*) FROM audit_record").await;
    push_count(&mut out, pool, "argo_skills_active_total", "Skills with no superseded_by",
        "SELECT COUNT(*) FROM skill WHERE superseded_by IS NULL").await;
    push_count(&mut out, pool, "argo_workspace_artifacts_total", "Recorded marketplace pushes",
        "SELECT COUNT(*) FROM marketplace_artifact").await;
    push_count(&mut out, pool, "argo_forge_runs_running",
        "Forge runs currently in flight (status='running')",
        "SELECT COUNT(*) FROM forge_run WHERE status = 'running'").await;
    push_count(&mut out, pool, "argo_workspaces_total",
        "Registered workspace rows",
        "SELECT COUNT(*) FROM workspace").await;
    push_count(&mut out, pool, "argo_memory_entries_total",
        "Active memory_entry rows (not superseded)",
        "SELECT COUNT(*) FROM memory_entry WHERE superseded_by IS NULL").await;
    push_count(&mut out, pool, "argo_judge_records_total",
        "Recorded judge_record rows",
        "SELECT COUNT(*) FROM judge_record").await;
    push_sum(&mut out, pool, "argo_spend_usd_micros_total",
        "Sum of spend_ledger.amount_usd_micros across every session",
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger").await;
    push_forge_runs_by_status(&mut out, pool).await;

    // Audit chain integrity — verify_all is more expensive than
    // a COUNT, so it gets its own block. Failure logs a comment
    // and emits 0 so the scrape still parses.
    match argo_audit::verify_all(store).await {
        Ok(report) => {
            let _ = writeln!(
                &mut out,
                "\n# HELP argo_audit_chain_ok Whether the audit chain verified end-to-end (1=ok, 0=broken)\n\
                 # TYPE argo_audit_chain_ok gauge\n\
                 argo_audit_chain_ok {}\n\
                 # HELP argo_audit_chain_breaks Number of chain breaks reported by verify_all\n\
                 # TYPE argo_audit_chain_breaks gauge\n\
                 argo_audit_chain_breaks {}",
                if report.ok { 1 } else { 0 },
                report.breaks.len(),
            );
        }
        Err(e) => {
            let _ = writeln!(
                &mut out,
                "\n# error: verify_all failed: {e}\n\
                 # TYPE argo_audit_chain_ok gauge\n\
                 argo_audit_chain_ok 0\n\
                 # TYPE argo_audit_chain_breaks gauge\n\
                 argo_audit_chain_breaks 0",
            );
        }
    }
    out
}

async fn push_count(
    out: &mut String,
    pool: &sqlx::SqlitePool,
    name: &str,
    help: &str,
    sql: &str,
) {
    let _ = writeln!(out, "\n# HELP {name} {help}\n# TYPE {name} counter");
    match sqlx::query_scalar::<_, i64>(sql).fetch_one(pool).await {
        Ok(n) => {
            let _ = writeln!(out, "{name} {n}");
        }
        Err(e) => {
            let _ = writeln!(out, "# error: {e}\n{name} 0");
        }
    }
}

async fn push_sum(
    out: &mut String,
    pool: &sqlx::SqlitePool,
    name: &str,
    help: &str,
    sql: &str,
) {
    let _ = writeln!(out, "\n# HELP {name} {help}\n# TYPE {name} counter");
    match sqlx::query_scalar::<_, i64>(sql).fetch_one(pool).await {
        Ok(n) => {
            let _ = writeln!(out, "{name} {n}");
        }
        Err(e) => {
            let _ = writeln!(out, "# error: {e}\n{name} 0");
        }
    }
}

async fn push_forge_runs_by_status(out: &mut String, pool: &sqlx::SqlitePool) {
    let _ = writeln!(
        out,
        "\n# HELP argo_forge_runs_total Recorded forge runs grouped by terminal status\n\
         # TYPE argo_forge_runs_total counter"
    );
    let rows: Result<Vec<(String, i64)>, _> =
        sqlx::query_as("SELECT status, COUNT(*) FROM forge_run GROUP BY status")
            .fetch_all(pool)
            .await;
    match rows {
        Ok(rows) if rows.is_empty() => {
            let _ = writeln!(out, "argo_forge_runs_total{{status=\"none\"}} 0");
        }
        Ok(rows) => {
            for (status, n) in rows {
                let safe = status.replace('"', "");
                let _ = writeln!(out, "argo_forge_runs_total{{status=\"{safe}\"}} {n}");
            }
        }
        Err(e) => {
            let _ = writeln!(
                out,
                "# error: forge_run group-by status failed: {e}\n\
                 argo_forge_runs_total{{status=\"unknown\"}} 0"
            );
        }
    }
}

/// Build the full HTTP response (status line + headers + body)
/// for `/metrics`. Returned as bytes so the caller can pipe it
/// straight into the TcpStream without going through the JSON-
/// flavoured response builder in `mod.rs`.
pub(super) async fn http_response(store: Option<&Arc<SessionStore>>) -> Vec<u8> {
    let body = render(store).await;
    let resp = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/plain; version=0.0.4; charset=utf-8\r\n\
         Content-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    resp.into_bytes()
}

/// Cheap path-prefix check. Mirrors the
/// `streaming::is_streaming_runs_request` pattern so the
/// listener loop can dispatch /metrics outside of the JSON
/// response builder.
pub(super) fn is_metrics_request(req: &str) -> bool {
    req.starts_with("GET /metrics ") || req.starts_with("GET /metrics?")
}

/// `POST /metrics/reset` — zero out the in-process scrape
/// counter and return `{previous, current}` as JSON. Useful
/// for tests + ops escape hatch for runaway pollers. Does not
/// touch boot time — uptime keeps climbing.
pub(super) fn metrics_reset_response() -> (&'static str, String) {
    let previous = reset_scrape_count();
    (
        "200 OK",
        format!("{{\"previous\":{previous},\"current\":0}}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn render_returns_well_formed_lines_when_store_unavailable() {
        let body = render(None).await;
        assert!(body.contains("argo_build_info{version="));
        assert!(body.contains("argo_store_available 0"));
        assert!(body.contains("# TYPE"));
    }

    #[tokio::test]
    async fn render_with_open_store_emits_count_lines() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(&dir.path().join("argo.db")).await.unwrap();
        store.migrate().await.unwrap();
        let store = Arc::new(store);
        let body = render(Some(&store)).await;
        assert!(body.contains("argo_store_available 1"));
        assert!(body.contains("argo_sessions_total 0"));
        assert!(body.contains("argo_messages_total 0"));
        assert!(body.contains("argo_skills_active_total 0"));
        assert!(body.contains("argo_audit_chain_ok"));
    }

    #[tokio::test]
    async fn http_response_has_correct_content_type() {
        let bytes = http_response(None).await;
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.starts_with("HTTP/1.1 200 OK"));
        assert!(s.contains("Content-Type: text/plain; version=0.0.4"));
    }

    #[test]
    fn is_metrics_request_detects_path_prefix() {
        assert!(is_metrics_request("GET /metrics HTTP/1.1\r\n"));
        assert!(is_metrics_request("GET /metrics?foo=bar HTTP/1.1\r\n"));
        assert!(!is_metrics_request("GET /metricsfoo HTTP/1.1\r\n"));
        assert!(!is_metrics_request("POST /metrics HTTP/1.1\r\n"));
    }
}
