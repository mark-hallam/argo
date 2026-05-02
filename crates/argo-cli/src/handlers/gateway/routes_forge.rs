//! `GET /forge/runs` and `GET /forge/runs/<id>` response builders.
//! Lifted out of `routes_meta.rs` so that file stays under the
//! 500-LOC budget. Pure read-side — no chain mutation.

use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /forge/runs/count[?status=&since-minutes=N]` — count of
/// recorded forge runs after filters. Mirrors the CLI's
/// `argo forge runs count`. Caps the fetch at 10,000 rows.
pub(super) async fn forge_runs_count_response(
    status: Option<&str>,
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match store.list_forge_runs(10_000).await {
        Ok(mut rows) => {
            if let Some(m) = since_minutes {
                let cutoff_micros =
                    (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
                rows.retain(|r| r.started_at >= cutoff_micros);
            }
            let n = match status {
                Some(want) => rows.iter().filter(|r| r.status == want).count(),
                None => rows.len(),
            };
            let payload = serde_json::json!({
                "count": n,
                "status": status,
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list_forge_runs: {e}\"}}"),
        ),
    }
}

/// `GET /forge/runs[?limit&since-minutes&status&score-min&score-max
/// &skill&has-candidate&iterations-min&iterations-max]` —
/// list recorded forge optimization runs. All flags AND'd
/// together. `has-candidate=true` keeps only rows whose
/// `candidate_version` is non-NULL (i.e. produced at least
/// one iteration's worth of mutation). `score-min` and
/// `score-max` define an inclusive band on `final_score`;
/// rows with `NULL` final_score never satisfy either.
/// `iterations-min` and `iterations-max` define an inclusive
/// band on the iteration count (a fresh run has 0).
#[allow(clippy::too_many_arguments)]
pub(super) async fn forge_runs_response(
    limit: u32,
    since_minutes: Option<i64>,
    status: Option<&str>,
    score_min: Option<f64>,
    score_max: Option<f64>,
    skill: Option<&str>,
    has_candidate: Option<bool>,
    iterations_min: Option<i64>,
    iterations_max: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match store.list_forge_runs(limit).await {
        Ok(rows) => {
            let cutoff_micros = since_minutes.map(|m| {
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros()
            });
            let filtered: Vec<argo_store::ForgeRunRow> = rows
                .into_iter()
                .filter(|r| match cutoff_micros {
                    Some(c) => r.started_at >= c,
                    None => true,
                })
                .filter(|r| match status {
                    Some(want) => r.status == want,
                    None => true,
                })
                .filter(|r| match score_min {
                    Some(threshold) => r.final_score.is_some_and(|s| s >= threshold),
                    None => true,
                })
                .filter(|r| match score_max {
                    Some(threshold) => r.final_score.is_some_and(|s| s <= threshold),
                    None => true,
                })
                .filter(|r| match skill {
                    Some(want) => r.skill_name == want,
                    None => true,
                })
                .filter(|r| match has_candidate {
                    Some(true) => r.candidate_version.is_some(),
                    Some(false) => r.candidate_version.is_none(),
                    None => true,
                })
                .filter(|r| match iterations_min {
                    Some(min) => r.iterations >= min,
                    None => true,
                })
                .filter(|r| match iterations_max {
                    Some(max) => r.iterations <= max,
                    None => true,
                })
                .collect();
            if matches!(format, Some("markdown")) {
                let n = filtered.len();
                let mut out = format!(
                    "# Forge runs ({n} match{})\n\n",
                    if n == 1 { "" } else { "es" }
                );
                if !filtered.is_empty() {
                    out.push_str(
                        "| status | id | skill | base | candidate | iters | spend (USD) | score |\n",
                    );
                    out.push_str(
                        "| :---: | --- | --- | --- | --- | ---: | ---: | ---: |\n",
                    );
                    for r in &filtered {
                        let id_short: String =
                            hex::encode(&r.id).chars().take(8).collect();
                        let cand =
                            r.candidate_version.clone().unwrap_or_else(|| "-".into());
                        let score = r
                            .final_score
                            .map(|s| format!("{s:.3}"))
                            .unwrap_or_else(|| "-".into());
                        let marker = if r.status == "running" { "●" } else { " " };
                        out.push_str(&format!(
                            "| {marker} | `{id_short}` | `{}` | `{}` | `{cand}` | {} | ${:.4} | {score} |\n",
                            r.skill_name,
                            r.base_version,
                            r.iterations,
                            r.spend_usd_micros as f64 / 1_000_000.0,
                        ));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = filtered
                .into_iter()
                .map(|r| {
                    // Mirror forge_run_show_response so list +
                    // single-row views agree on the elapsed math:
                    // running rows count from now, ended rows
                    // count from ended_at - started_at.
                    let now_micros = chrono::Utc::now().timestamp_micros();
                    let end_micros = r.ended_at.unwrap_or(now_micros);
                    let elapsed_seconds = ((end_micros - r.started_at) / 1_000_000).max(0);
                    serde_json::json!({
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
            format!("{{\"error\":\"list_forge_runs: {e}\"}}"),
        ),
    }
}

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

/// `GET /forge/runs/<id>/payload` — return the queued payload
/// JSON the daemon needs to (re)run a forge_run row. 404
/// when no row matches OR the run has no `forge_queue`
/// payload (typically because it was created via
/// `?in_process=true` and never had a queue entry).
pub(super) async fn forge_run_payload_response(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
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
            // Re-emit as a wrapped JSON object so the response
            // is always parseable as JSON regardless of what's
            // inside the payload string.
            let parsed: serde_json::Value =
                serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null);
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
