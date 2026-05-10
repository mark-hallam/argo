//! Audit-related GET handlers, extracted from `routes_post.rs`
//! to keep that file under the 500-LOC budget. Pure read-side —
//! no chain mutation here. The dispatcher in `routes.rs` still
//! owns URL parsing.

use super::routes_query::{parse_query_kv, parse_since_minutes_from_get};
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

// Session-scoped audit handlers were split into `routes_audit_session.rs`
// to keep this file under the 500-LOC ceiling. Re-export so external
// callers (`super::routes_audit::audit_session_response`, etc.)
// continue to resolve identically.
pub(super) use super::routes_audit_session::{
    audit_session_response, audit_session_verify_response,
};

/// `GET /audit/recent` dispatch — parses 4 query params then
/// hands off to [`audit_recent_response`]. Lives here so the
/// dispatcher in routes.rs stays under the LOC budget.
pub(super) async fn dispatch_audit_recent(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let limit: u32 = parse_query_kv(tail, "limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);
    let kind = parse_query_kv(tail, "kind");
    let since = parse_since_minutes_from_get(req, "/audit/recent");
    let session = parse_query_kv(tail, "session");
    let format = parse_query_kv(tail, "format");
    audit_recent_response(
        limit,
        kind.as_deref(),
        since,
        session.as_deref(),
        store,
        format.as_deref(),
    )
    .await
}

/// `GET /audit/count[?kind=…&session=<uuid>&since-minutes=N]` —
/// count of audit rows after filters. Useful for "how many
/// tool_calls in the last hour" monitoring queries that don't
/// need the row bodies. Caps the underlying fetch at 10,000.
pub(super) async fn audit_count_response(
    kind: Option<&str>,
    since_minutes: Option<i64>,
    session: Option<&str>,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let since_micros = since_minutes.map(|m| {
        (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros()
    });
    let session_bytes = match session {
        Some(s) => match uuid::Uuid::parse_str(s) {
            Ok(u) => Some(u.as_bytes().to_vec()),
            Err(e) => {
                return (
                    "400 Bad Request",
                    format!("{{\"error\":\"invalid session id: {e}\"}}"),
                );
            }
        },
        None => None,
    };
    let filters = argo_audit::TailFilters {
        kind,
        since_micros,
        session_id: session_bytes.as_deref(),
    };
    match argo_audit::tail_rows_filtered(store, 10_000, &filters).await {
        Ok(rows) => {
            let payload = serde_json::json!({
                "count": rows.len(),
                "kind": kind,
                "session": session,
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"audit count: {e}\"}}"),
        ),
    }
}

/// `GET /audit/recent?limit=N&kind=<k>&since-minutes=<m>&session=<id>` —
/// most recent N audit rows across every session, in seq order
/// (oldest first inside the window). Mirrors `argo audit show
/// --tail N`. Defaults to 50.
///
/// All filters apply at SQL level before the LIMIT, so e.g.
/// `?limit=10&kind=tool_call&since-minutes=60&session=<uuid>`
/// returns up to 10 tool-call rows from one session in the last
/// hour — not 10-most-recent-trimmed.
#[allow(clippy::too_many_arguments)]
pub(super) async fn audit_recent_response(
    limit: u32,
    kind: Option<&str>,
    since_minutes: Option<i64>,
    session: Option<&str>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let since_micros = since_minutes.map(|m| {
        (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros()
    });
    let session_bytes = match session {
        Some(s) => match uuid::Uuid::parse_str(s) {
            Ok(u) => Some(u.as_bytes().to_vec()),
            Err(e) => {
                return (
                    "400 Bad Request",
                    format!("{{\"error\":\"invalid session id: {e}\"}}"),
                );
            }
        },
        None => None,
    };
    let filters = argo_audit::TailFilters {
        kind,
        since_micros,
        session_id: session_bytes.as_deref(),
    };
    match argo_audit::tail_rows_filtered(store, limit, &filters).await {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                let mut out = format!(
                    "# Audit recent ({n} row{})\n\n",
                    if n == 1 { "" } else { "s" }
                );
                if !rows.is_empty() {
                    // #691: vertical column lets operators triage
                    // by binding without drilling into each row.
                    out.push_str("| seq | kind | session_id | vertical |\n");
                    out.push_str("| ---: | --- | --- | --- |\n");
                    for r in &rows {
                        let sid = r
                            .session_id
                            .as_deref()
                            .unwrap_or("-");
                        let vert = r.vertical_id.as_deref().unwrap_or("-");
                        out.push_str(&format!(
                            "| {} | `{}` | `{sid}` | `{vert}` |\n",
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
            format!("{{\"error\":\"audit tail: {e}\"}}"),
        ),
    }
}

/// `GET /audit/by-kind[?since-minutes=N&format=markdown]` —
/// histogram of audit rows by `kind`. Useful for at-a-glance
/// "what's happening" — heavy ToolCall counts vs. sparse
/// TurnEnd, etc. Optionally filtered to the last N minutes.
/// Returns `{by_kind: {kind: count}}`; markdown returns a
/// 2-column table sorted by count descending.
pub(super) async fn audit_by_kind_response(
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let pool = store.pool();
    let rows: Result<Vec<(String, i64)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT kind, COUNT(*) FROM audit_record WHERE created_at >= ? \
                 GROUP BY kind ORDER BY kind",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT kind, COUNT(*) FROM audit_record GROUP BY kind ORDER BY kind",
            )
            .fetch_all(pool)
            .await
        }
    };
    match rows {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let mut sorted = rows.clone();
                sorted.sort_by_key(|r| std::cmp::Reverse(r.1));
                let total: i64 = sorted.iter().map(|(_, n)| *n).sum();
                let mut out = format!(
                    "# Audit by kind ({} rows across {} kinds)\n\n",
                    total,
                    sorted.len()
                );
                if !sorted.is_empty() {
                    out.push_str("| kind | count |\n");
                    out.push_str("| --- | ---: |\n");
                    for (k, n) in &sorted {
                        out.push_str(&format!("| `{k}` | {n} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let mut by_kind = serde_json::Map::new();
            for (k, n) in rows {
                by_kind.insert(k, serde_json::json!(n));
            }
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({
                    "by_kind": serde_json::Value::Object(by_kind),
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"audit_by_kind: {e}\"}}"),
        ),
    }
}

/// `GET /audit/by-day[?days=N&format=markdown]` — daily audit
/// row rollup. Same shape as `/sessions/by-day` and
/// `/messages/by-day`. Useful for spotting busy days vs.
/// quiet ones in the audit chain. Default 30 days, cap 365.
pub(super) async fn audit_by_day_response(
    days: u32,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let days = days.clamp(1, 365);
    let cutoff_micros =
        (chrono::Utc::now() - chrono::Duration::days(i64::from(days))).timestamp_micros();
    let sql = "SELECT \
                 strftime('%Y-%m-%d', created_at / 1000000, 'unixepoch') AS bucket, \
                 COUNT(*) AS n \
               FROM audit_record WHERE created_at >= ? \
               GROUP BY bucket ORDER BY bucket ASC";
    match sqlx::query_as::<_, (String, i64)>(sql)
        .bind(cutoff_micros)
        .fetch_all(store.pool())
        .await
    {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                let mut out = format!(
                    "# Audit by day ({n} day{} with activity, last {days} days)\n\n",
                    if n == 1 { "" } else { "s" }
                );
                if !rows.is_empty() {
                    out.push_str("| day | count |\n");
                    out.push_str("| --- | ---: |\n");
                    for (b, c) in &rows {
                        out.push_str(&format!("| `{b}` | {c} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(b, c)| {
                    serde_json::json!({
                        "bucket": b,
                        "count": c,
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({"by_day": arr}))
                    .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"audit_by_day: {e}\"}}"),
        ),
    }
}
