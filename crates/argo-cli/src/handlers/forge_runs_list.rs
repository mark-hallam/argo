//! `argo forge runs count` and `argo forge runs list` — query and
//! render forge_run rows. Split out of `forge.rs` to keep the
//! dispatcher under the 500-LOC ceiling.
use super::forge_subject::open_store;
use argo_types::{ArgoError, ArgoResult};

/// `argo forge runs count [--status] [--json]` — print just the
/// count of recorded forge runs. Caps the fetch at 10,000 rows
/// for safety.
pub(super) async fn count_runs(
    status: Option<String>,
    since_minutes: Option<i64>,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    let mut rows = store.list_forge_runs(10_000).await?;
    if let Some(want) = status.as_deref() {
        rows.retain(|r| r.status == want);
    }
    if let Some(m) = since_minutes {
        let cutoff_micros = (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
        rows.retain(|r| r.started_at >= cutoff_micros);
    }
    let n = rows.len();
    if json {
        let payload = serde_json::json!({
            "count": n,
            "status": status,
            "since_minutes": since_minutes,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!("{n}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn list_runs(
    limit: u32,
    json: bool,
    format: String,
    since_minutes: Option<i64>,
    status: Option<String>,
    score_min: Option<f64>,
    score_max: Option<f64>,
    skill: Option<String>,
    has_candidate: Option<bool>,
    iterations_min: Option<i64>,
    iterations_max: Option<i64>,
    sort_by: String,
) -> ArgoResult<()> {
    let store = open_store().await?;
    let mut rows = store.list_forge_runs(limit).await?;
    if let Some(m) = since_minutes {
        let cutoff_micros = (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
        rows.retain(|r| r.started_at >= cutoff_micros);
    }
    if let Some(want) = status.as_deref() {
        rows.retain(|r| r.status == want);
    }
    if let Some(min) = score_min {
        rows.retain(|r| r.final_score.is_some_and(|s| s >= min));
    }
    if let Some(max) = score_max {
        rows.retain(|r| r.final_score.is_some_and(|s| s <= max));
    }
    if let Some(want) = skill.as_deref() {
        rows.retain(|r| r.skill_name == want);
    }
    if let Some(want) = has_candidate {
        rows.retain(|r| r.candidate_version.is_some() == want);
    }
    if let Some(lo) = iterations_min {
        rows.retain(|r| r.iterations >= lo);
    }
    if let Some(hi) = iterations_max {
        rows.retain(|r| r.iterations <= hi);
    }
    match sort_by.as_str() {
        "started-at" => {} // store returns reverse-chronological
        "score" => {
            // NULLs sink to the bottom; otherwise descending.
            rows.sort_by(|a, b| match (b.final_score, a.final_score) {
                (Some(bs), Some(as_)) => bs.partial_cmp(&as_).unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        }
        "spend" => rows.sort_by_key(|r| std::cmp::Reverse(r.spend_usd_micros)),
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown --sort-by '{other}' — use started-at|score|spend"
            )));
        }
    }
    let pick = if json { "json" } else { format.as_str() };
    match pick {
        "json" => {
            for r in &rows {
                let line = serde_json::json!({
                    "id": hex::encode(&r.id),
                    "skill_name": r.skill_name,
                    "started_at": r.started_at,
                    "ended_at": r.ended_at,
                    "base_version": r.base_version,
                    "candidate_version": r.candidate_version,
                    "iterations": r.iterations,
                    "spend_usd_micros": r.spend_usd_micros,
                    "final_score": r.final_score,
                    "status": r.status,
                });
                println!("{}", serde_json::to_string(&line).unwrap_or_default());
            }
            return Ok(());
        }
        "markdown" => {
            println!(
                "# Forge runs ({} match{})",
                rows.len(),
                if rows.len() == 1 { "" } else { "es" }
            );
            println!();
            if rows.is_empty() {
                return Ok(());
            }
            println!(
                "| status | id | skill | base | candidate | iters | spend (USD) | score |"
            );
            println!("| :---: | --- | --- | --- | --- | ---: | ---: | ---: |");
            for r in &rows {
                let id_short: String = hex::encode(&r.id).chars().take(8).collect();
                let cand = r.candidate_version.clone().unwrap_or_else(|| "-".into());
                let score = r
                    .final_score
                    .map(|s| format!("{s:.3}"))
                    .unwrap_or_else(|| "-".into());
                let spend = format!("${:.4}", r.spend_usd_micros as f64 / 1_000_000.0);
                let marker = if r.status == "running" { "●" } else { " " };
                println!(
                    "| {marker} | `{id_short}` | `{}` | `{}` | `{cand}` | {} | {spend} | {score} |",
                    r.skill_name, r.base_version, r.iterations
                );
            }
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    if rows.is_empty() {
        println!("(no forge runs recorded yet)");
        return Ok(());
    }
    println!(
        "  id        skill                    base       candidate  iters  spend USD  score   status"
    );
    for r in rows {
        let cand = r.candidate_version.unwrap_or_else(|| "-".into());
        let score = r
            .final_score
            .map(|s| format!("{s:.3}"))
            .unwrap_or_else(|| "-".into());
        let spend = format!("${:.2}", r.spend_usd_micros as f64 / 1_000_000.0);
        let id_short: String = hex::encode(&r.id).chars().take(8).collect();
        // Running rows get a ● prefix so the user can see at a
        // glance which runs are still in flight (mirrors the TUI
        // F-overlay marker).
        let marker = if r.status == "running" { "●" } else { " " };
        println!(
            "{marker} {id_short:<8}  {name:<24} {base:<10} {cand:<10} {iters:<6} {spend:<10} {score:<7} {status}",
            name = r.skill_name,
            base = r.base_version,
            cand = cand,
            iters = r.iterations,
            spend = spend,
            score = score,
            status = r.status
        );
    }
    Ok(())
}
