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

/// `GET /forge/runs/<id>/audit[?format=markdown]` — audit rows
/// linked to one forge run via the payload's `$.run_id` field.
/// Forge runs emit `forge.run.start` + `forge.run.end` rows
/// (plus per-iter rows for the LLM-driven optimizer); this
/// endpoint returns all of them ordered by seq ASC. Useful for
/// "what actually happened on this forge run?" without grepping
/// the full audit chain.
pub(super) async fn forge_run_audit_response(
    run_id: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    if uuid::Uuid::parse_str(run_id).is_err() {
        return (
            "400 Bad Request",
            format!("{{\"error\":\"invalid forge run id: {run_id}\"}}"),
        );
    }
    let pool = store.pool();
    let rows: Result<Vec<(i64, String, String, i64)>, _> = sqlx::query_as(
        "SELECT seq, kind, payload_json, created_at FROM audit_record \
         WHERE json_extract(payload_json, '$.run_id') = ? \
         ORDER BY seq ASC",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await;
    match rows {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                let mut out = format!(
                    "# Audit rows for forge run `{run_id}` ({n} row{})\n\n",
                    if n == 1 { "" } else { "s" }
                );
                if !rows.is_empty() {
                    out.push_str("| seq | kind | created_at_micros |\n");
                    out.push_str("| ---: | --- | ---: |\n");
                    for (seq, kind, _payload, ts) in &rows {
                        out.push_str(&format!("| {seq} | `{kind}` | {ts} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(seq, kind, payload_json, ts)| {
                    serde_json::json!({
                        "seq": seq,
                        "kind": kind,
                        "payload": serde_json::from_str::<serde_json::Value>(&payload_json)
                            .unwrap_or(serde_json::Value::Null),
                        "created_at_micros": ts,
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({
                    "run_id": run_id,
                    "rows": arr,
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"forge_run_audit: {e}\"}}"),
        ),
    }
}
