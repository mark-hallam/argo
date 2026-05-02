//! `GET /costs/by-day` — daily spend rollup. Lifted out of
//! `routes_post.rs` so that file stays under the 500-LOC
//! budget. Reads `spend_ledger` directly via `store.pool()`;
//! no write-side concerns.
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

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
