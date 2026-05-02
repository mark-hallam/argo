//! Session-rollup handlers (`sessions/by-day`,
//! `sessions/by-provider`) lifted out of `routes_top.rs`
//! so it stays under the 500-LOC ceiling.
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /sessions/by-day[?days=N&format=markdown]` — daily
/// session-creation count against the `session` table for the
/// last N days (default 30; cap 365). Returns
/// `{by_day: [{bucket, count}]}` ordered chronologically;
/// days with no sessions are omitted. Markdown returns a
/// 2-column table.
pub(super) async fn sessions_by_day_response(
    days: u32,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let days = days.clamp(1, 365);
    let cutoff_micros =
        (chrono::Utc::now() - chrono::Duration::days(i64::from(days))).timestamp_micros();
    let sql = "SELECT \
                 strftime('%Y-%m-%d', started_at / 1000000, 'unixepoch') AS bucket, \
                 COUNT(*) AS n \
               FROM session WHERE started_at >= ? \
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
                    "# Sessions by day ({n} day{} with activity, last {days} days)\n\n",
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
            format!("{{\"error\":\"sessions_by_day: {e}\"}}"),
        ),
    }
}

/// `GET /sessions/by-provider[?since-minutes=N&format=markdown]`
/// — histogram of session rows grouped by `provider`. Pairs
/// with `/costs/by-model` for "which providers/models are
/// burning what". Returns `{by_provider: {provider: count}}`
/// JSON or a 2-column markdown table sorted desc.
pub(super) async fn sessions_by_provider_response(
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let pool = store.pool();
    let rows: Result<Vec<(String, i64)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT provider, COUNT(*) FROM session WHERE started_at >= ? \
                 GROUP BY provider ORDER BY provider",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT provider, COUNT(*) FROM session GROUP BY provider ORDER BY provider",
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
                    "# Sessions by provider ({total} sessions across {} providers)\n\n",
                    sorted.len()
                );
                if !sorted.is_empty() {
                    out.push_str("| provider | count |\n");
                    out.push_str("| --- | ---: |\n");
                    for (p, n) in &sorted {
                        out.push_str(&format!("| `{p}` | {n} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let mut by_provider = serde_json::Map::new();
            for (p, n) in rows {
                by_provider.insert(p, serde_json::json!(n));
            }
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({
                    "by_provider": serde_json::Value::Object(by_provider),
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"sessions_by_provider: {e}\"}}"),
        ),
    }
}

/// `GET /sessions/by-model[?since-minutes=N&format=markdown]`
/// — histogram of session rows grouped by `model`. Symmetric
/// to `/sessions/by-provider` (#594) — rounds out the by-*
/// family for sessions. Returns
/// `{by_model: {model: count}}` JSON or a 2-column markdown
/// table sorted desc.
pub(super) async fn sessions_by_model_response(
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let pool = store.pool();
    let rows: Result<Vec<(String, i64)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT model, COUNT(*) FROM session WHERE started_at >= ? \
                 GROUP BY model ORDER BY model",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT model, COUNT(*) FROM session GROUP BY model ORDER BY model",
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
                    "# Sessions by model ({total} sessions across {} models)\n\n",
                    sorted.len()
                );
                if !sorted.is_empty() {
                    out.push_str("| model | count |\n");
                    out.push_str("| --- | ---: |\n");
                    for (m, n) in &sorted {
                        out.push_str(&format!("| `{m}` | {n} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let mut by_model = serde_json::Map::new();
            for (m, n) in rows {
                by_model.insert(m, serde_json::json!(n));
            }
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({
                    "by_model": serde_json::Value::Object(by_model),
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"sessions_by_model: {e}\"}}"),
        ),
    }
}
