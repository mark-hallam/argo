//! `GET /costs/by-day` + `GET /costs/recent` — costs surface
//! handlers lifted out of `routes_post.rs` so that file stays
//! under the 500-LOC budget. Reads `spend_ledger` directly via
//! `store.pool()`; no write-side concerns.
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /costs/recent[?limit=N&since-minutes=M&format=markdown]`
/// — workspace-wide most-recent spend ledger entries, ordered
/// by `charged_at_micros DESC`. Parallel to `/audit/recent`
/// (#406) and `/messages/recent` (#619) — completes the recent
/// tail family for the costs surface.
/// Default limit 50, capped at 500. Returns
/// `[{seq, session_id, provider, model, amount_usd_micros, charged_at_micros}]`.
pub(super) async fn costs_recent_response(
    limit: u32,
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let limit = limit.clamp(1, 500);
    let pool = store.pool();
    let rows: Result<Vec<(i64, Vec<u8>, i64, i64, String, String)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT seq, session_id, charged_at_micros, amount_usd_micros, provider, model \
                 FROM spend_ledger WHERE charged_at_micros >= ? \
                 ORDER BY charged_at_micros DESC LIMIT ?",
            )
            .bind(cutoff)
            .bind(i64::from(limit))
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT seq, session_id, charged_at_micros, amount_usd_micros, provider, model \
                 FROM spend_ledger ORDER BY charged_at_micros DESC LIMIT ?",
            )
            .bind(i64::from(limit))
            .fetch_all(pool)
            .await
        }
    };
    match rows {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                let mut out = format!(
                    "# Costs recent ({n} ledger row{})\n\n",
                    if n == 1 { "" } else { "s" }
                );
                if !rows.is_empty() {
                    out.push_str("| seq | session_id | provider / model | USD |\n");
                    out.push_str("| ---: | --- | --- | ---: |\n");
                    for (seq, sid_b, _ts, amt, prov, model) in &rows {
                        let sid = uuid::Uuid::from_slice(sid_b)
                            .map(|u| u.to_string())
                            .unwrap_or_else(|_| hex::encode(sid_b));
                        out.push_str(&format!(
                            "| {seq} | `{sid}` | `{prov} / {model}` | ${:.4} |\n",
                            *amt as f64 / 1_000_000.0
                        ));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(seq, sid_b, ts, amt, prov, model)| {
                    let sid = uuid::Uuid::from_slice(&sid_b)
                        .map(|u| u.to_string())
                        .unwrap_or_else(|_| hex::encode(&sid_b));
                    serde_json::json!({
                        "seq": seq,
                        "session_id": sid,
                        "charged_at_micros": ts,
                        "amount_usd_micros": amt,
                        "provider": prov,
                        "model": model,
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
            format!("{{\"error\":\"costs_recent: {e}\"}}"),
        ),
    }
}

/// `GET /costs/by-day[?days=N&format=markdown]` — daily spend
/// rollup against the last N days (default 30; cap 365).
/// Returns `{by_day: [{bucket, spent_usd_micros}]}` ordered
/// chronologically (oldest first). Days with no spend are
/// omitted — the caller can fill them in if a sparse-friendly
/// chart is needed. Markdown returns a 2-column table with the
/// total formatted in dollars.
pub(super) async fn costs_by_day_response(
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
                 strftime('%Y-%m-%d', charged_at_micros / 1000000, 'unixepoch') AS bucket, \
                 COALESCE(SUM(amount_usd_micros), 0) AS total \
               FROM spend_ledger WHERE charged_at_micros >= ? \
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
                    "# Spend by day ({n} day{} with activity, last {days} days)\n\n",
                    if n == 1 { "" } else { "s" }
                );
                if !rows.is_empty() {
                    out.push_str("| day | total (USD) |\n");
                    out.push_str("| --- | ---: |\n");
                    for (b, t) in &rows {
                        out.push_str(&format!(
                            "| `{b}` | ${:.4} |\n",
                            *t as f64 / 1_000_000.0
                        ));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(b, t)| {
                    serde_json::json!({
                        "bucket": b,
                        "spent_usd_micros": u64::try_from(t).unwrap_or(0),
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
            format!("{{\"error\":\"costs_by_day: {e}\"}}"),
        ),
    }
}
