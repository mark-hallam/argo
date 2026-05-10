//! Web HTML view of the swarm fleet (`claude-dashboard-fleet-view-html`).
//!
//! Mounted at `GET /dashboard/fleet` by [`super::routes::dispatch_route`].
//! Lists every worker that has any audit activity, with a status badge,
//! the timestamp of its most recent action, and the ids/kinds of its
//! last 5 actions. Read-only; auto-refreshes every 10s via an inline
//! `setTimeout(() => location.reload(), 10_000)` block — no external
//! CSS/JS, no template engine.
//!
//! ## Sensible deviations from the brief
//!
//! 1. The brief says the handler signature is
//!    `pub async fn handle_dashboard_fleet(state: Arc<GatewayState>)
//!    -> impl IntoResponse`. The actual gateway is a custom HTTP/1.1
//!    dispatcher returning `(status_line, body)` tuples (see
//!    `dashboard.rs` for the same constraint), so the handler conforms
//!    to that shape.
//! 2. The brief mentions a "worker registry / sessions store" lookup.
//!    There is no in-process worker registry; the canonical signal of
//!    worker activity is the audit log. Querying `audit_record` for
//!    rows whose `payload_json` carries a `worker_id` gives the same
//!    answer (every worker that has ever taken an action shows up).
//!    Workers with zero audit rows are by definition not "active" and
//!    are excluded — matching the brief's headline question.
//! 3. The brief allows lifting the HTML escape helper to a shared
//!    module if cleaner. To respect the may-touch rule that says the
//!    existing dashboard handler must not be edited, this module
//!    duplicates the small `esc()` helper rather than refactoring
//!    `dashboard/render.rs`.

use argo_store::SessionStore;
use std::sync::Arc;

const HTML_HDR: &str =
    "200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store";

/// True if `req` targets `GET /dashboard/fleet` (with or without
/// query string). Trailing slash variants are accepted to match
/// browser URL-bar typos.
pub(super) fn is_fleet(req: &str) -> bool {
    let Some(line) = req.lines().next() else {
        return false;
    };
    let Some(after) = line.strip_prefix("GET ") else {
        return false;
    };
    let Some(path) = after.split([' ', '?']).next() else {
        return false;
    };
    matches!(path, "/dashboard/fleet" | "/dashboard/fleet/")
}

/// Render the fleet HTML page. Returns the gateway's `(status, body)`
/// tuple. When `store` is `None` (gateway booted without a session
/// store, e.g. in tests), the page renders a clean "no workers"
/// state rather than 500-ing.
pub(super) async fn handle(store: Option<&Arc<SessionStore>>) -> (&'static str, String) {
    let workers = match store {
        Some(s) => query_fleet(s).await,
        None => Vec::new(),
    };
    (HTML_HDR, render_fleet(&workers))
}

/// One row of the fleet view: a worker plus its recent activity.
#[derive(Debug, Clone)]
pub(super) struct WorkerSnapshot {
    pub(super) worker_id: String,
    pub(super) status: WorkerStatus,
    pub(super) last_kind: String,
    pub(super) last_at_micros: i64,
    /// Most recent first.
    pub(super) recent: Vec<RecentAction>,
}

#[derive(Debug, Clone)]
pub(super) struct RecentAction {
    pub(super) action_id: String,
    pub(super) kind: String,
    pub(super) at_micros: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkerStatus {
    Idle,
    Running,
    Blocked,
    Error,
}

impl WorkerStatus {
    fn css_class(self) -> &'static str {
        match self {
            Self::Idle => "fleet-badge-idle",
            Self::Running => "fleet-badge-running",
            Self::Blocked => "fleet-badge-blocked",
            Self::Error => "fleet-badge-error",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Blocked => "blocked",
            Self::Error => "error",
        }
    }
}

/// Pull at most 500 recent audit rows that carry a `worker_id`,
/// group them by worker, and compute the per-worker status. 500 is
/// enough to cover ~50 active workers × 10 actions each before
/// deeper history would be wanted; the page caps the displayed
/// recent list at 5 per worker anyway.
pub(super) async fn query_fleet(store: &SessionStore) -> Vec<WorkerSnapshot> {
    let rows: Vec<(String, String, Vec<u8>, i64)> = sqlx::query_as(
        "SELECT json_extract(payload_json, '$.worker_id'), kind, id, created_at \
         FROM audit_record \
         WHERE json_extract(payload_json, '$.worker_id') IS NOT NULL \
         ORDER BY seq DESC LIMIT 500",
    )
    .fetch_all(store.pool())
    .await
    .unwrap_or_default();
    group_rows_into_snapshots(rows)
}

fn group_rows_into_snapshots(
    rows: Vec<(String, String, Vec<u8>, i64)>,
) -> Vec<WorkerSnapshot> {
    let mut by_worker: std::collections::BTreeMap<String, Vec<RecentAction>> =
        std::collections::BTreeMap::new();
    for (worker_id, kind, id_bytes, created_at) in rows {
        if worker_id.is_empty() {
            continue;
        }
        let action_id = id_bytes.iter().map(|b| format!("{b:02x}")).collect();
        by_worker
            .entry(worker_id)
            .or_default()
            .push(RecentAction { action_id, kind, at_micros: created_at });
    }
    let mut out: Vec<WorkerSnapshot> = by_worker
        .into_iter()
        .map(|(worker_id, mut actions)| {
            actions.sort_by(|a, b| b.at_micros.cmp(&a.at_micros));
            let (last_kind, last_at) = actions
                .first()
                .map(|a| (a.kind.clone(), a.at_micros))
                .unwrap_or_default();
            let status = classify_status(&last_kind);
            actions.truncate(5);
            WorkerSnapshot {
                worker_id,
                status,
                last_kind,
                last_at_micros: last_at,
                recent: actions,
            }
        })
        .collect();
    out.sort_by(|a, b| b.last_at_micros.cmp(&a.last_at_micros));
    out
}

fn classify_status(last_kind: &str) -> WorkerStatus {
    let lower = last_kind.to_lowercase();
    if lower.contains("error") || lower.contains("fail") {
        WorkerStatus::Error
    } else if lower.contains("approval") || lower.contains("blocked") || lower.contains("pending")
    {
        WorkerStatus::Blocked
    } else if lower.is_empty() {
        WorkerStatus::Idle
    } else {
        WorkerStatus::Running
    }
}

pub(super) fn render_fleet(workers: &[WorkerSnapshot]) -> String {
    let body = if workers.is_empty() {
        "<p class=\"fleet-empty\">No workers have reported activity yet.</p>".to_string()
    } else {
        let rows: String = workers.iter().map(render_worker_card).collect();
        format!("<div class=\"fleet-grid\">{rows}</div>")
    };
    render_layout(&body)
}

fn render_worker_card(w: &WorkerSnapshot) -> String {
    let last_at = format_ts(w.last_at_micros);
    let recent = if w.recent.is_empty() {
        "<li class=\"fleet-recent-empty\">no activity</li>".to_string()
    } else {
        w.recent
            .iter()
            .map(|a| {
                let when = format_ts(a.at_micros);
                format!(
                    "<li><span class=\"fleet-when\">{when}</span> \
                     <span class=\"fleet-kind\">{kind}</span> \
                     <code class=\"fleet-aid\">{aid}</code></li>",
                    when = esc(&when),
                    kind = esc(&a.kind),
                    aid = esc(&short(&a.action_id, 12)),
                )
            })
            .collect::<String>()
    };
    format!(
        "<section class=\"fleet-card\">\
           <header><code class=\"fleet-wid\">{wid}</code>\
             <span class=\"fleet-badge {cls}\">{status}</span></header>\
           <p class=\"fleet-meta\">last: <strong>{kind}</strong> at {at}</p>\
           <ul class=\"fleet-recent\">{recent}</ul>\
         </section>",
        wid = esc(&w.worker_id),
        cls = w.status.css_class(),
        status = w.status.label(),
        kind = esc(&w.last_kind),
        at = esc(&last_at),
        recent = recent,
    )
}

fn render_layout(body: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <title>Argo Fleet</title>\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <style>{css}</style></head>\
         <body><header class=\"fleet-page-header\"><h1>Fleet</h1>\
         <p class=\"fleet-sub\">Every worker, with status and last 5 actions. \
         Auto-refreshes every 10s.</p></header><main>{body}</main>\
         <script>setTimeout(function(){{location.reload();}}, 10000);</script>\
         </body></html>",
        css = INLINE_CSS,
        body = body,
    )
}

const INLINE_CSS: &str = "\
:root{--bg:#0e1116;--fg:#e6edf3;--muted:#7d8590;--card:#161b22;--accent:#58a6ff}\
*{box-sizing:border-box}\
body{margin:0;font:14px/1.5 -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;\
background:var(--bg);color:var(--fg)}\
.fleet-page-header{padding:24px;border-bottom:1px solid #30363d}\
.fleet-page-header h1{margin:0 0 4px 0;font-size:20px}\
.fleet-sub{margin:0;color:var(--muted);font-size:13px}\
main{padding:24px}\
.fleet-empty{color:var(--muted)}\
.fleet-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(320px,1fr));gap:16px}\
.fleet-card{background:var(--card);border:1px solid #30363d;border-radius:6px;\
padding:14px}\
.fleet-card header{display:flex;justify-content:space-between;align-items:center;\
margin-bottom:8px;gap:8px}\
.fleet-wid{font-size:13px;color:var(--accent);overflow:hidden;text-overflow:ellipsis}\
.fleet-meta{margin:0 0 8px 0;color:var(--muted);font-size:12px}\
.fleet-meta strong{color:var(--fg)}\
.fleet-badge{display:inline-block;padding:2px 8px;border-radius:10px;font-size:11px;\
font-weight:600;text-transform:uppercase}\
.fleet-badge-idle{background:#484f58;color:#fff}\
.fleet-badge-running{background:#1f6feb;color:#fff}\
.fleet-badge-blocked{background:#d29922;color:#000}\
.fleet-badge-error{background:#da3633;color:#fff}\
.fleet-recent{list-style:none;padding:0;margin:0;font-size:12px}\
.fleet-recent li{padding:2px 0;display:flex;gap:8px;align-items:baseline}\
.fleet-when{color:var(--muted);font-variant-numeric:tabular-nums;flex-shrink:0}\
.fleet-kind{flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}\
.fleet-aid{color:var(--muted);font-size:11px}\
.fleet-recent-empty{color:var(--muted);font-style:italic}\
";

pub(super) fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) fn short(s: &str, n: usize) -> String {
    if s.len() > n {
        format!("{}…", &s[..n])
    } else {
        s.to_string()
    }
}

pub(super) fn format_ts(micros: i64) -> String {
    if micros <= 0 {
        return "—".to_string();
    }
    chrono::DateTime::<chrono::Utc>::from_timestamp_micros(micros)
        .map(|d| d.format("%Y-%m-%d %H:%M:%SZ").to_string())
        .unwrap_or_else(|| "—".to_string())
}

#[cfg(test)]
#[path = "dashboard_fleet_tests.rs"]
mod tests;
