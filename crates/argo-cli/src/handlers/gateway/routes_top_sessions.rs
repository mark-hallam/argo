//! `GET /sessions` family handlers (list + count) lifted out
//! of `routes_top.rs` so it stays under the 500-LOC ceiling.

use super::routes_query::{parse_query_kv, parse_since_minutes_from_get};
use super::store_unavailable;
use super::streaming::{stream_json_array, stream_ndjson, WireFmt};
use argo_store::SessionStore;
use std::sync::Arc;
use tokio::net::TcpStream;

/// `GET /sessions` dispatcher: parse all 8 query params then
/// hand off to [`sessions_response`]. Lives here so the
/// signature drift on sessions_response stays local —
/// routes.rs just calls `dispatch_sessions(req, store)`.
pub(super) async fn dispatch_sessions(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let since = parse_since_minutes_from_get(req, "/sessions");
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let older_than =
        parse_query_kv(tail, "older-than-minutes").and_then(|v| v.parse::<i64>().ok());
    let status = parse_query_kv(tail, "status");
    let provider = parse_query_kv(tail, "provider");
    let model = parse_query_kv(tail, "model");
    let title_contains = parse_query_kv(tail, "title-contains").map(|s| s.replace('+', " "));
    let spent_min = parse_query_kv(tail, "spent-min").and_then(|v| v.parse::<i64>().ok());
    let spent_max = parse_query_kv(tail, "spent-max").and_then(|v| v.parse::<i64>().ok());
    let limit: u32 = parse_query_kv(tail, "limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let format = parse_query_kv(tail, "format");
    sessions_response(
        store,
        since,
        older_than,
        status.as_deref(),
        provider.as_deref(),
        model.as_deref(),
        title_contains.as_deref(),
        spent_min,
        spent_max,
        limit,
        format.as_deref(),
    )
    .await
}

/// `GET /sessions` — list every session row joined with each
/// session's spend total. Optional filters: `since-minutes`
/// (newer than), `older-than-minutes` (older than), `status=
/// open|ended`, `provider=<id>`, `model=<id>`,
/// `title-contains=<sub>`, `spent-min=<usd_micros>`,
/// `spent-max=<usd_micros>`, `limit=<N>` (default 100),
/// `format=markdown` (markdown table render, paste-able).
#[allow(clippy::too_many_arguments)]
pub(super) async fn sessions_response(
    store: Option<&Arc<SessionStore>>,
    since_minutes: Option<i64>,
    older_than_minutes: Option<i64>,
    status: Option<&str>,
    provider: Option<&str>,
    model: Option<&str>,
    title_contains: Option<&str>,
    spent_min: Option<i64>,
    spent_max: Option<i64>,
    limit: u32,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    match store.list_all_sessions(limit).await {
        Ok(mut rows) => {
            if let Some(minutes) = since_minutes {
                let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes);
                rows.retain(|s| s.started_at >= cutoff);
            }
            if let Some(minutes) = older_than_minutes {
                let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes);
                rows.retain(|s| s.started_at < cutoff);
            }
            if let Some(want) = status {
                rows.retain(|s| match want {
                    "open" => s.ended_at.is_none(),
                    "ended" => s.ended_at.is_some(),
                    _ => true,
                });
            }
            if let Some(want) = provider {
                rows.retain(|s| s.provider == want);
            }
            if let Some(want) = model {
                rows.retain(|s| s.model == want);
            }
            if let Some(needle) = title_contains {
                let lower = needle.to_lowercase();
                rows.retain(|s| {
                    s.title
                        .as_deref()
                        .map(|t| t.to_lowercase().contains(&lower))
                        .unwrap_or(false)
                });
            }
            let spend_by_sid = super::spend_per_session(store).await.unwrap_or_default();
            // Spend filters apply after the cheap filters above so
            // we don't pay the lookup unless we still have rows.
            // Treat negative `?spent-min` / `?spent-max` as 0 since
            // usd_micros is non-negative by construction.
            let min_u = spent_min.map(|v| u64::try_from(v).unwrap_or(0));
            let max_u = spent_max.map(|v| u64::try_from(v).unwrap_or(0));
            if min_u.is_some() || max_u.is_some() {
                rows.retain(|s| {
                    let spent = spend_by_sid
                        .get(s.id.0.as_bytes().as_slice())
                        .copied()
                        .unwrap_or(0);
                    if let Some(min) = min_u {
                        if spent < min {
                            return false;
                        }
                    }
                    if let Some(max) = max_u {
                        if spent > max {
                            return false;
                        }
                    }
                    true
                });
            }
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                let mut out = format!(
                    "# Sessions ({n} match{})\n\n",
                    if n == 1 { "" } else { "es" }
                );
                if !rows.is_empty() {
                    // #690: vertical column completes the operator
                    // trifecta (rollup + show + list). Unbound rows
                    // render as `-` to keep the table compact.
                    out.push_str("| status | session_id | provider | model | vertical | spend (USD) | title |\n");
                    out.push_str("| :---: | --- | --- | --- | --- | ---: | --- |\n");
                    for s in &rows {
                        let spent = spend_by_sid
                            .get(s.id.0.as_bytes().as_slice())
                            .copied()
                            .unwrap_or(0);
                        let marker = if s.ended_at.is_none() { "●" } else { "○" };
                        let title = s.title.as_deref().unwrap_or("").replace('|', "\\|");
                        let vertical = s.vertical_id.as_deref().unwrap_or("-");
                        out.push_str(&format!(
                            "| {marker} | `{}` | `{}` | `{}` | `{vertical}` | ${:.4} | {title} |\n",
                            s.id.0,
                            s.provider,
                            s.model,
                            spent as f64 / 1_000_000.0,
                        ));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|s| {
                    let spent = spend_by_sid
                        .get(s.id.0.as_bytes().as_slice())
                        .copied()
                        .unwrap_or(0);
                    // #690: vertical_id always present (null for
                    // unbound) so clients don't have to special-case
                    // a missing key.
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "model": s.model,
                        "provider": s.provider,
                        "title": s.title,
                        "vertical_id": s.vertical_id,
                        "started_at": s.started_at.to_rfc3339(),
                        "ended_at": s.ended_at.map(|d| d.to_rfc3339()),
                        "spent_usd_micros": spent,
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
            format!("{{\"error\":\"list sessions: {e}\"}}"),
        ),
    }
}

/// `GET /sessions/count[?status=…&since-minutes=N&older-than-minutes=N&provider=…&model=…]`
/// — count of session rows after filters. Caps fetch at 10k.
#[allow(clippy::too_many_arguments)]
pub(super) async fn sessions_count_response(
    status: Option<&str>,
    since_minutes: Option<i64>,
    older_than_minutes: Option<i64>,
    provider: Option<&str>,
    model: Option<&str>,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    match store.list_all_sessions(10_000).await {
        Ok(mut rows) => {
            if let Some(m) = since_minutes {
                let cutoff = chrono::Utc::now() - chrono::Duration::minutes(m);
                rows.retain(|s| s.started_at >= cutoff);
            }
            if let Some(m) = older_than_minutes {
                let cutoff = chrono::Utc::now() - chrono::Duration::minutes(m);
                rows.retain(|s| s.started_at < cutoff);
            }
            if let Some(want) = provider {
                rows.retain(|s| s.provider == want);
            }
            if let Some(want) = model {
                rows.retain(|s| s.model == want);
            }
            let n = match status {
                Some("open") => rows.iter().filter(|s| s.ended_at.is_none()).count(),
                Some("ended") => rows.iter().filter(|s| s.ended_at.is_some()).count(),
                Some("all") | None => rows.len(),
                Some(other) => {
                    return (
                        "400 Bad Request",
                        format!(
                            "{{\"error\":\"unknown status '{other}' — use open|ended|all\"}}"
                        ),
                    );
                }
            };
            let payload = serde_json::json!({
                "count": n,
                "status": status.unwrap_or("all"),
                "provider": provider,
                "model": model,
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list_all_sessions: {e}\"}}"),
        ),
    }
}

/// Streaming variant of `GET /sessions`. Same per-row JSON shape as
/// the buffered path so JSON-array consumers parse it identically.
/// Filters apply incrementally as rows flow out, so a 50k-row
/// result-set never materialises as one giant String.
pub(super) async fn stream_sessions(
    req: &str,
    store: &Arc<SessionStore>,
    out: &mut TcpStream,
    fmt: WireFmt,
) -> tokio::io::Result<()> {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let since = parse_since_minutes_from_get(req, "/sessions");
    let older_than =
        parse_query_kv(tail, "older-than-minutes").and_then(|v| v.parse::<i64>().ok());
    let status = parse_query_kv(tail, "status");
    let provider = parse_query_kv(tail, "provider");
    let model = parse_query_kv(tail, "model");
    let title_contains =
        parse_query_kv(tail, "title-contains").map(|s| s.replace('+', " "));
    let spent_min = parse_query_kv(tail, "spent-min").and_then(|v| v.parse::<i64>().ok());
    let spent_max = parse_query_kv(tail, "spent-max").and_then(|v| v.parse::<i64>().ok());
    let limit: u32 = parse_query_kv(tail, "limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);

    // Sessions live behind a single indexed query already. We pull
    // them once, then stream rows out filter-by-filter — the win is
    // skipping the giant `serde_json::to_string(&Vec)` allocation
    // that was the buffered path's true memory peak.
    let rows = match store.list_all_sessions(limit).await {
        Ok(r) => r,
        Err(e) => {
            return write_stream_error(out, "500 Internal Server Error", &format!("list sessions: {e}"))
                .await;
        }
    };
    let spend_by_sid = super::spend_per_session(store).await.unwrap_or_default();
    let now = chrono::Utc::now();
    let since_cutoff = since.map(|m| now - chrono::Duration::minutes(m));
    let older_cutoff = older_than.map(|m| now - chrono::Duration::minutes(m));
    let title_lower = title_contains.as_deref().map(str::to_lowercase);
    let min_u = spent_min.map(|v| u64::try_from(v).unwrap_or(0));
    let max_u = spent_max.map(|v| u64::try_from(v).unwrap_or(0));

    let stream = futures::stream::iter(rows.into_iter().filter_map(move |s| {
        if let Some(c) = since_cutoff {
            if s.started_at < c {
                return None;
            }
        }
        if let Some(c) = older_cutoff {
            if s.started_at >= c {
                return None;
            }
        }
        if let Some(want) = status.as_deref() {
            let keep = match want {
                "open" => s.ended_at.is_none(),
                "ended" => s.ended_at.is_some(),
                _ => true,
            };
            if !keep {
                return None;
            }
        }
        if let Some(want) = provider.as_deref() {
            if s.provider != want {
                return None;
            }
        }
        if let Some(want) = model.as_deref() {
            if s.model != want {
                return None;
            }
        }
        if let Some(needle) = title_lower.as_deref() {
            let hit = s
                .title
                .as_deref()
                .map(|t| t.to_lowercase().contains(needle))
                .unwrap_or(false);
            if !hit {
                return None;
            }
        }
        let spent = spend_by_sid
            .get(s.id.0.as_bytes().as_slice())
            .copied()
            .unwrap_or(0);
        if let Some(min) = min_u {
            if spent < min {
                return None;
            }
        }
        if let Some(max) = max_u {
            if spent > max {
                return None;
            }
        }
        Some(Ok(serde_json::json!({
            "id": s.id.0.to_string(),
            "model": s.model,
            "provider": s.provider,
            "title": s.title,
            "vertical_id": s.vertical_id,
            "started_at": s.started_at.to_rfc3339(),
            "ended_at": s.ended_at.map(|d| d.to_rfc3339()),
            "spent_usd_micros": spent,
        })))
    }));
    match fmt {
        WireFmt::JsonArray => stream_json_array(out, stream).await,
        WireFmt::Ndjson => stream_ndjson(out, stream).await,
    }
}

async fn write_stream_error(
    out: &mut TcpStream,
    status: &str,
    msg: &str,
) -> tokio::io::Result<()> {
    use tokio::io::AsyncWriteExt;
    let body = format!("{{\"error\":\"{msg}\"}}");
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    out.write_all(resp.as_bytes()).await?;
    out.flush().await
}
