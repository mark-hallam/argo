//! Forge-run rollup handlers (`forge/runs/by-skill`) — lifted
//! out of `routes_forge.rs` to keep that file under the
//! 500-LOC ceiling once the rollup family grew.
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /forge/runs/by-skill[?since-minutes=N&format=markdown]`
/// — histogram of forge_run rows grouped by `skill_name`.
/// Useful for "which skills are getting tuned most often".
/// Returns `{by_skill: {skill: count}}` JSON or a 2-column
/// markdown table sorted desc.
pub(super) async fn forge_runs_by_skill_response(
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
                "SELECT skill_name, COUNT(*) FROM forge_run WHERE started_at >= ? \
                 GROUP BY skill_name ORDER BY skill_name",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT skill_name, COUNT(*) FROM forge_run \
                 GROUP BY skill_name ORDER BY skill_name",
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
                    "# Forge runs by skill ({total} runs across {} skills)\n\n",
                    sorted.len()
                );
                if !sorted.is_empty() {
                    out.push_str("| skill | count |\n");
                    out.push_str("| --- | ---: |\n");
                    for (s, n) in &sorted {
                        out.push_str(&format!("| `{s}` | {n} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let mut by_skill = serde_json::Map::new();
            for (s, n) in rows {
                by_skill.insert(s, serde_json::json!(n));
            }
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({
                    "by_skill": serde_json::Value::Object(by_skill),
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"forge_runs_by_skill: {e}\"}}"),
        ),
    }
}
