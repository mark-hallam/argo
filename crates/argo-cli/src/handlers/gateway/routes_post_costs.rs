//! GET /costs/{by-model,by-provider,by-session} response builder
//! split out of `routes_post.rs` for the 500-LOC budget. (The
//! parent file's name is historical — these are heavy query
//! helpers, not POSTs; the dispatcher in routes_dispatch_costs.rs
//! is unchanged.)
//!
//! Public API stays stable: `routes_post.rs` re-exports
//! `costs_breakdown_response` so existing call sites compile
//! unchanged.
use argo_store::SessionStore;
use std::sync::Arc;

use super::store_unavailable;

pub(super) async fn costs_breakdown_response(
    mode: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let sql = match mode {
        "by_model" => {
            "SELECT provider || ' / ' || model AS bucket, \
             COALESCE(SUM(amount_usd_micros), 0) AS total \
             FROM spend_ledger GROUP BY provider, model ORDER BY total DESC LIMIT 50"
        }
        "by_session" => {
            "SELECT lower(hex(session_id)) AS bucket, \
             COALESCE(SUM(amount_usd_micros), 0) AS total \
             FROM spend_ledger GROUP BY session_id ORDER BY total DESC LIMIT 50"
        }
        "by_provider" => {
            "SELECT provider AS bucket, \
             COALESCE(SUM(amount_usd_micros), 0) AS total \
             FROM spend_ledger GROUP BY provider ORDER BY total DESC LIMIT 50"
        }
        _ => unreachable!(),
    };
    match sqlx::query_as::<_, (String, i64)>(sql)
        .fetch_all(store.pool())
        .await
    {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let title = match mode {
                    "by_model" => "Spend by model",
                    "by_session" => "Spend by session",
                    "by_provider" => "Spend by provider",
                    _ => "Spend",
                };
                let n = rows.len();
                let mut out = format!("# {title} ({n} bucket{})\n\n", if n == 1 { "" } else { "s" });
                if !rows.is_empty() {
                    out.push_str("| bucket | total (USD) |\n");
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
                serde_json::to_string(&serde_json::json!({mode: arr}))
                    .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"costs breakdown: {e}\"}}"),
        ),
    }
}
