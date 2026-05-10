//! Session-scoped audit GET handlers, extracted from
//! `routes_audit.rs` to keep that file under the 500-LOC budget.
//! Both endpoints take a session id and either verify the chain or
//! list the rows; pure read-side, no chain mutation here.

use super::store_unavailable;
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

/// `GET /audit/sessions/<id>/verify[?include-breaks=true]` —
/// verify the audit chain for one session. Default body shape
/// is `{ok, rows, breaks}` where `breaks` is the count.
/// With `?include-breaks=true` the response gains a
/// `breaks_detail: [{seq, kind, reason}, …]` array. Mirrors
/// `argo audit verify --session <id>`. 200 in both the
/// chain-ok and chain-broken cases — `ok` says which.
pub(super) async fn audit_session_verify_response(
    sid: &str,
    include_breaks: bool,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    match argo_audit::verify_session(store, id).await {
        Ok(report) => {
            if matches!(format, Some("markdown")) {
                let mut out = format!("# Audit verify — session `{sid}`\n\n");
                out.push_str(&format!(
                    "- **Status**: {}\n",
                    if report.ok { "OK ✓" } else { "**BROKEN**" }
                ));
                out.push_str(&format!("- **Rows**: {}\n", report.rows));
                out.push_str(&format!("- **Breaks**: {}\n", report.breaks.len()));
                if include_breaks && !report.breaks.is_empty() {
                    out.push_str("\n## Break detail\n\n");
                    out.push_str("| seq | kind | reason |\n");
                    out.push_str("| ---: | --- | --- |\n");
                    for b in &report.breaks {
                        let reason = b.reason.replace('|', "\\|");
                        out.push_str(&format!("| {} | `{}` | {reason} |\n", b.seq, b.kind));
                    }
                }
                return ("200 OK", out);
            }
            let mut payload = serde_json::json!({
                "ok": report.ok,
                "rows": report.rows,
                "breaks": report.breaks.len(),
            });
            if include_breaks {
                let detail: Vec<serde_json::Value> = report
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
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("breaks_detail".to_string(), serde_json::Value::Array(detail));
                }
            }
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"verify_session: {e}\"}}"),
        ),
    }
}

/// `GET /audit/sessions/<id>?kind=<kind>&since-minutes=<m>` —
/// list audit rows scoped to one session, in `seq` order.
/// Mirrors `argo audit show --session <id>`. Filters apply
/// after fetching the full session window (sessions are
/// typically small, so this keeps the SQL trivial).
#[allow(clippy::too_many_arguments)]
pub(super) async fn audit_session_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    kind: Option<&str>,
    since_minutes: Option<i64>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    match argo_audit::rows_for_session(store, id).await {
        Ok(mut rows) => {
            if let Some(want) = kind {
                rows.retain(|r| r.kind == want);
            }
            if let Some(m) = since_minutes {
                let cutoff_micros = (chrono::Utc::now() - chrono::Duration::minutes(m))
                    .timestamp_micros();
                rows.retain(|r| r.created_at_micros >= cutoff_micros);
            }
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                // #691: surface the bound vertical in the heading
                // when present. Within one session every row shares
                // the same vertical so a per-row column would be
                // pure repetition.
                let vert = rows
                    .iter()
                    .find_map(|r| r.vertical_id.as_deref());
                let header_suffix = match vert {
                    Some(v) => format!(" — vertical `{v}`"),
                    None => String::new(),
                };
                let mut out = format!(
                    "# Audit chain for session `{sid}` ({n} row{}){header_suffix}\n\n",
                    if n == 1 { "" } else { "s" }
                );
                if !rows.is_empty() {
                    out.push_str("| seq | kind | hash |\n");
                    out.push_str("| ---: | --- | --- |\n");
                    for r in &rows {
                        let hash_short: String =
                            r.self_hash.chars().take(12).collect();
                        out.push_str(&format!(
                            "| {} | `{}` | `{hash_short}…` |\n",
                            r.seq, r.kind
                        ));
                    }
                }
                return ("200 OK", out);
            }
            (
                "200 OK",
                serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"audit session: {e}\"}}"),
        ),
    }
}
