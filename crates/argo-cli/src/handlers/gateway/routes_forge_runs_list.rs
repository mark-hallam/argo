//! `GET /forge/runs` and `GET /forge/runs/count` — list and count
//! response builders. Lifted out of `routes_forge.rs` to keep
//! that file under the 500-LOC budget.

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
