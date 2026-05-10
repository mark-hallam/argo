//! `GET /forge/runs/...` response builders. Pure read-side —
//! no chain mutation. List/count handlers live in the sibling
//! `routes_forge_runs_list` module so this file stays under
//! the 500-LOC budget; re-exports below keep the public call
//! shape (`super::routes_forge::fn_name`) unchanged.

use argo_store::SessionStore;
use std::sync::Arc;

pub(super) use super::routes_forge_runs_list::{
    forge_runs_count_response, forge_runs_response,
};

/// `GET /forge/runs/<id>` — fetch one forge_run row by id (mirrors
/// `argo forge runs show <id>`). Returns 404 when no row matches,
/// 503 when the store is unavailable, 400 when the id isn't a
/// valid UUID.
/// `GET /forge/runs/<id>[?format=markdown]` — single forge run
/// row. Default JSON shape (`{id, skill_name, started_at,
/// ended_at, elapsed_seconds, base_version, candidate_version,
/// iterations, spend_usd_micros, final_score, status}`).
/// With `?format=markdown`, returns a paste-able doc.
pub(super) async fn forge_run_show_response_with_format(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(raw_id) {
        Ok(u) => u,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid forge run id: {e}\"}}"),
            );
        }
    };
    match store.get_forge_run(id).await {
        Ok(Some(r)) => {
            // For running rows: time-since-start; for ended
            // rows: ended_at - started_at. Both in whole seconds.
            let now_micros = chrono::Utc::now().timestamp_micros();
            let end_micros = r.ended_at.unwrap_or(now_micros);
            let elapsed_seconds = ((end_micros - r.started_at) / 1_000_000).max(0);
            if matches!(format, Some("markdown")) {
                let cand = r.candidate_version.clone().unwrap_or_else(|| "(none)".into());
                let score = r
                    .final_score
                    .map(|s| format!("{s:.3}"))
                    .unwrap_or_else(|| "-".into());
                let mut out = format!(
                    "# Forge run `{}`\n\n",
                    hex::encode(&r.id).chars().take(12).collect::<String>()
                );
                out.push_str(&format!("- **Skill**: `{}`\n", r.skill_name));
                out.push_str(&format!("- **Status**: `{}`\n", r.status));
                out.push_str(&format!("- **Base**: `{}`\n", r.base_version));
                out.push_str(&format!("- **Candidate**: `{cand}`\n"));
                out.push_str(&format!("- **Iterations**: {}\n", r.iterations));
                out.push_str(&format!(
                    "- **Spend**: ${:.4}\n",
                    r.spend_usd_micros as f64 / 1_000_000.0
                ));
                out.push_str(&format!("- **Score**: {score}\n"));
                out.push_str(&format!("- **Elapsed**: {elapsed_seconds}s\n"));
                return ("200 OK", out);
            }
            let body = serde_json::json!({
                "id": hex::encode(&r.id),
                "skill_name": r.skill_name,
                "started_at": r.started_at,
                "ended_at": r.ended_at,
                "elapsed_seconds": elapsed_seconds,
                "base_version": r.base_version,
                "candidate_version": r.candidate_version,
                "iterations": r.iterations,
                "spend_usd_micros": r.spend_usd_micros,
                "final_score": r.final_score,
                "status": r.status,
            });
            (
                "200 OK",
                serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Ok(None) => (
            "404 Not Found",
            format!("{{\"error\":\"no forge run with id {raw_id}\"}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"get_forge_run: {e}\"}}"),
        ),
    }
}

/// `GET /forge/runs/by-day[?days=N&format=markdown]` — daily
/// forge run start count against `forge_run.started_at` for
/// the last N days (default 30; cap 365). Returns
/// `{by_day: [{bucket, count}]}` ordered chronologically.
/// Markdown returns a 2-column table.
pub(super) async fn forge_runs_by_day_response(
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
               FROM forge_run WHERE started_at >= ? \
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
                    "# Forge runs by day ({n} day{} with activity, last {days} days)\n\n",
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
            format!("{{\"error\":\"forge_runs_by_day: {e}\"}}"),
        ),
    }
}

/// `GET /forge/runs/by-status[?since-minutes=N&format=markdown]`
/// — histogram of forge_run rows grouped by `status`.
/// Returns `{by_status: {status: count}}` JSON or a 2-column
/// markdown table sorted by count descending. Mirrors the
/// `argo_forge_runs_total{status=…}` series in /metrics, but
/// exposed as a dedicated read endpoint.
pub(super) async fn forge_runs_by_status_response(
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
                "SELECT status, COUNT(*) FROM forge_run WHERE started_at >= ? \
                 GROUP BY status ORDER BY status",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT status, COUNT(*) FROM forge_run GROUP BY status ORDER BY status",
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
                    "# Forge runs by status ({total} rows across {} status values)\n\n",
                    sorted.len()
                );
                if !sorted.is_empty() {
                    out.push_str("| status | count |\n");
                    out.push_str("| --- | ---: |\n");
                    for (s, n) in &sorted {
                        out.push_str(&format!("| `{s}` | {n} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let mut by_status = serde_json::Map::new();
            for (s, n) in rows {
                by_status.insert(s, serde_json::json!(n));
            }
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({
                    "by_status": serde_json::Value::Object(by_status),
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"forge_runs_by_status: {e}\"}}"),
        ),
    }
}

/// `GET /forge/runs/<id>/payload[?format=markdown]` — return
/// the queued payload JSON the daemon needs to (re)run a
/// forge_run row. JSON default; `?format=markdown` returns a
/// paste-able doc with run_id header + a fenced JSON block
/// for the payload. 404 when no row matches OR the run has
/// no `forge_queue` payload (typically because it was created
/// via `?in_process=true` and never had a queue entry).
pub(super) async fn forge_run_payload_response(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(raw_id) {
        Ok(u) => u,
        Err(_) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid forge run id '{raw_id}'\"}}"),
            );
        }
    };
    match store.get_forge_queue_payload(id).await {
        Ok(Some(payload)) => {
            let parsed: serde_json::Value =
                serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null);
            if matches!(format, Some("markdown")) {
                let mut out = format!("# Payload for forge run `{id}`\n\n");
                out.push_str("```json\n");
                let pretty =
                    serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| payload.clone());
                out.push_str(&pretty);
                out.push_str("\n```\n");
                return ("200 OK", out);
            }
            let resp = serde_json::json!({
                "run_id": id.to_string(),
                "payload": parsed,
            });
            (
                "200 OK",
                serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Ok(None) => (
            "404 Not Found",
            format!("{{\"error\":\"no forge_queue payload for run {raw_id}\"}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"get_forge_queue_payload: {e}\"}}"),
        ),
    }
}
