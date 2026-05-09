//! Top-level read handlers extracted out of `routes.rs` to keep
//! the dispatcher under the 500-LOC budget. None of these touch
//! the streaming machinery — they're plain `(status_line, body)`
//! responders for the largest GET endpoints.
//!
//! `GET /sessions` list/count handlers live in the
//! `routes_top_sessions` sibling, re-exported below so existing
//! call sites (`super::routes_top::{…}`) keep working.

use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) use super::routes_top_sessions::{dispatch_sessions, sessions_count_response};

/// `GET /doctor[?format=markdown]` — JSON snapshot of
/// `argo doctor` by default; markdown returns a paste-able
/// status doc (workspace + DB + audit + creds sections).
pub(super) async fn doctor_response(
    format: Option<&str>,
) -> (&'static str, String) {
    match crate::handlers::doctor::collect().await {
        Ok(v) => {
            if matches!(format, Some("markdown")) {
                return ("200 OK", render_doctor_markdown(&v));
            }
            (
                "200 OK",
                serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"doctor: {e}\"}}"),
        ),
    }
}

/// Render the doctor JSON payload as a paste-able markdown
/// status doc. Subset of the CLI's `--format markdown` view —
/// hits the workspace + db + audit fields most ops want at
/// a glance.
fn render_doctor_markdown(payload: &serde_json::Value) -> String {
    use std::fmt::Write;
    let cwd = payload["cwd"].as_str().unwrap_or("?");
    let profile = payload["profile"].as_str().unwrap_or("?");
    let toml_path = payload["argo_toml"]["path"].as_str().unwrap_or("?");
    let toml_ok = payload["argo_toml"]["ok"].as_bool().unwrap_or(false);
    let home_path = payload["argo_home"]["path"].as_str().unwrap_or("?");
    let home_ok = payload["argo_home"]["ok"].as_bool().unwrap_or(false);
    let mut out = String::new();
    let _ = writeln!(out, "# argo doctor\n");
    let _ = writeln!(out, "## Workspace\n");
    let _ = writeln!(out, "- **cwd**: `{cwd}`");
    let _ = writeln!(
        out,
        "- **argo.toml**: `{toml_path}` {}",
        if toml_ok { "✓" } else { "_(missing)_" }
    );
    let _ = writeln!(
        out,
        "- **ARGO_HOME**: `{home_path}` {}",
        if home_ok { "✓" } else { "_(missing)_" }
    );
    let _ = writeln!(out, "- **profile**: `{profile}`");
    if let Some(db) = payload.get("db") {
        let _ = writeln!(out, "\n## DB\n");
        if let Some(v) = db.get("schema_version").and_then(|v| v.as_u64()) {
            let _ = writeln!(out, "- **schema_version**: {v}");
        }
        if let Some(rows) = db.get("rows").and_then(|v| v.as_object()) {
            let _ = writeln!(out, "- **rows**:");
            for (k, v) in rows {
                let _ = writeln!(out, "  - {k}: {v}");
            }
        }
    }
    if let Some(audit) = payload.get("audit") {
        let _ = writeln!(out, "\n## Audit\n");
        let ok = audit.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        let _ = writeln!(
            out,
            "- **chain ok**: {}",
            if ok { "✓" } else { "**broken**" }
        );
        if let Some(rows) = audit.get("rows").and_then(|v| v.as_u64()) {
            let _ = writeln!(out, "- **rows**: {rows}");
        }
        if let Some(breaks) = audit.get("breaks").and_then(|v| v.as_u64()) {
            let _ = writeln!(out, "- **breaks**: {breaks}");
        }
    }
    out
}

/// `GET /health/full` — deep health (chain + skill + memory).
/// Mirrors `argo health`: returns 500 if `ok=false`, 200 otherwise.
pub(super) async fn health_full_response() -> (&'static str, String) {
    match crate::handlers::health::collect().await {
        Ok(v) => {
            let status = if v["ok"].as_bool().unwrap_or(false) {
                "200 OK"
            } else {
                "500 Internal Server Error"
            };
            (
                status,
                serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"health: {e}\"}}"),
        ),
    }
}

/// `GET /audit[?format=markdown]` — chain summary: rows,
/// breaks, kind histogram. Mirrors `argo audit verify`.
/// Sorted by descending count then ascending kind label so
/// the response is stable across runs. Markdown returns
/// `# Audit log` doc with status, rows, breaks, and kinds
/// histogram table.
pub(super) async fn audit_response(
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let report = match argo_audit::verify_all(store).await {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"verify_all: {e}\"}}"),
            );
        }
    };
    let mut kinds: Vec<(String, usize)> = report
        .kind_counts
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    kinds.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if matches!(format, Some("markdown")) {
        let mut out = String::from("# Audit log\n\n");
        out.push_str(&format!("- **Total rows**: {}\n", report.rows));
        out.push_str(&format!(
            "- **Status**: {}\n",
            if report.ok { "OK ✓" } else { "**BROKEN**" }
        ));
        out.push_str(&format!("- **Breaks**: {}\n", report.breaks.len()));
        if !kinds.is_empty() {
            out.push_str("\n## Histogram\n\n");
            out.push_str("| kind | count |\n");
            out.push_str("| --- | ---: |\n");
            for (k, v) in &kinds {
                out.push_str(&format!("| `{k}` | {v} |\n"));
            }
        }
        return ("200 OK", out);
    }
    let kinds_obj: serde_json::Map<String, serde_json::Value> = kinds
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::from(v)))
        .collect();
    let payload = serde_json::json!({
        "ok": report.ok,
        "rows": report.rows,
        "breaks": report.breaks.len(),
        "kinds": kinds_obj,
    });
    (
        "200 OK",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// `GET /audit/breaks` — actual ChainBreak rows (not just the
/// count). Each break is `{seq, kind, reason}`. Useful for
/// debugging a broken chain — `argo audit verify --json`
/// already exposes this on the CLI side; this is the gateway
/// equivalent.
pub(super) async fn audit_breaks_response(
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    match argo_audit::verify_all(store).await {
        Ok(report) => {
            if matches!(format, Some("markdown")) {
                let n = report.breaks.len();
                let mut out = format!(
                    "# Audit chain breaks ({n} break{})\n\n",
                    if n == 1 { "" } else { "s" }
                );
                if !report.breaks.is_empty() {
                    out.push_str("| seq | kind | reason |\n");
                    out.push_str("| ---: | --- | --- |\n");
                    for b in &report.breaks {
                        let reason = b.reason.replace('|', "\\|");
                        out.push_str(&format!("| {} | `{}` | {reason} |\n", b.seq, b.kind));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = report
                .breaks
                .iter()
                .map(|b| {
                    serde_json::json!({
                        "seq": b.seq,
                        "kind": b.kind,
                        "reason": b.reason,
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"verify_all: {e}\"}}"),
        ),
    }
}
