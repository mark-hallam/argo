//! `argo forge` — runs list / show / abort + promote / bench /
//! optimize. Optimize lives in `forge_optimize.rs` so this file
//! stays under 500 LOC; the shared `BenchSubject` +
//! `open_store` helpers live in `forge_subject.rs`.
use super::forge_optimize;
use super::forge_subject::{open_store, BenchSubject};
use crate::cli::{ForgeCmd, ForgeRunsCmd};
use argo_audit::{AuditKind, AuditSink};
use argo_eval::{load_jsonl, run_eval};
use argo_skill::SkillStore;
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;
use std::sync::Arc;

pub async fn run(cmd: ForgeCmd) -> ArgoResult<()> {
    match cmd {
        ForgeCmd::Runs { cmd } => match cmd {
            ForgeRunsCmd::Count {
                status,
                since_minutes,
                json,
            } => count_runs(status, since_minutes, json).await,
            ForgeRunsCmd::List {
                limit,
                json,
                format,
                since_minutes,
                status,
                score_min,
                score_max,
                skill,
                has_candidate,
                iterations_min,
                iterations_max,
                sort_by,
            } => {
                list_runs(
                    limit,
                    json,
                    format,
                    since_minutes,
                    status,
                    score_min,
                    score_max,
                    skill,
                    has_candidate,
                    iterations_min,
                    iterations_max,
                    sort_by,
                )
                .await
            }
            ForgeRunsCmd::Show { id } => super::forge_admin::show_run(&id).await,
            ForgeRunsCmd::Abort { id } => super::forge_admin::abort_run(&id).await,
            ForgeRunsCmd::AbortAll => super::forge_admin::abort_all_runs().await,
        },
        ForgeCmd::Promote { skill, version } => promote(&skill, &version).await,
        ForgeCmd::Bench {
            skill,
            examples,
            provider,
            model,
        } => bench(&skill, &examples, &provider, &model).await,
        ForgeCmd::Optimize {
            skill,
            examples,
            iterations,
            budget,
            stream,
        } => forge_optimize::run(&skill, &examples, iterations, budget, stream).await,
    }
}

async fn bench(
    skill_name: &str,
    examples_path: &str,
    provider: &str,
    model: &str,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let store = Arc::new(open_store().await?);
    let skills = SkillStore::new(Arc::clone(&store));

    // Resolve skill (lookup is informational — body isn't yet
    // injected into the prompt; that lands with the skill-loader
    // PR).
    let active = skills
        .find_active_by_name(skill_name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{skill_name}'")))?;

    let tasks = load_jsonl(&PathBuf::from(examples_path)).await?;
    if tasks.is_empty() {
        return Err(ArgoError::Provider(format!(
            "no eval tasks parsed from {examples_path}"
        )));
    }
    println!(
        "argo forge bench: {skill_name} v{} → {} task(s) from {examples_path}",
        active.frontmatter.version,
        tasks.len()
    );

    // Bootstrap the workspace bookkeeping so each bench session
    // can claim a tenant + workspace row.
    let ws_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let ws_root = cwd.to_string_lossy();
    let (tenant, workspace) = store.ensure_workspace(ws_name, &ws_root).await?;

    let run_id = store
        .start_forge_run(
            skill_name,
            &active.frontmatter.version,
            tasks.len() as u32,
        )
        .await?;
    let _ = AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunStart,
            serde_json::json!({
                "run_id": run_id.to_string(),
                "skill_name": skill_name,
                "examples": examples_path,
                "n": tasks.len(),
            }),
        )
        .await;

    let subject = BenchSubject {
        store: Arc::clone(&store),
        cwd: cwd.clone(),
        tenant,
        workspace,
        provider: provider.to_string(),
        model: model.to_string(),
        skill_name: skill_name.to_string(),
    };
    let (records, summary) = run_eval(&subject, &tasks).await;

    let status = if summary.errors > 0 && summary.errors == summary.n {
        "failed"
    } else if summary.errors > 0 {
        "partial"
    } else {
        "succeeded"
    };
    let _ = store
        .finish_forge_run(run_id, status, Some(summary.mean_score), summary.n as i64)
        .await;
    let _ = AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunEnd,
            serde_json::json!({
                "run_id": run_id.to_string(),
                "status": status,
                "mean_score": summary.mean_score,
                "pass_rate": summary.pass_rate,
                "errors": summary.errors,
            }),
        )
        .await;

    println!("\n  task                     score  passed  output (truncated)");
    for r in &records {
        let pass_tag = if r.passed { "yes" } else { "no" };
        let preview: String = r.output.chars().take(48).collect();
        println!(
            "  {name:<24} {score:.3}  {pass:<6}  {preview}",
            name = r.task,
            score = r.score,
            pass = pass_tag,
        );
    }
    println!(
        "\nsummary: n={} mean_score={:.3} pass_rate={:.3} errors={} (run_id={})",
        summary.n, summary.mean_score, summary.pass_rate, summary.errors, run_id
    );
    Ok(())
}


/// Async kick-off used by the gateway's `POST /forge/bench`.
/// Spawns a Tokio task that runs the bench; the resulting
/// forge_run row shows up in `GET /forge/runs`. Errors land in
/// the gateway log — the caller already got 202 Accepted.
pub(crate) fn spawn_bench(skill: String, examples: String, provider: String, model: String) {
    tokio::spawn(async move {
        if let Err(e) = bench(&skill, &examples, &provider, &model).await {
            eprintln!("argo gateway: forge bench ({skill}) failed: {e}");
        }
    });
}

async fn promote(skill_name: &str, version: &str) -> ArgoResult<()> {
    let store = Arc::new(open_store().await?);
    let skills = SkillStore::new(Arc::clone(&store));
    let promoted = skills.promote(skill_name, version).await?;
    AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunEnd,
            serde_json::json!({
                "kind": "promote",
                "skill_name": skill_name,
                "version": version,
                "skill_id": promoted.id.0.to_string(),
                "by": "cli",
            }),
        )
        .await?;
    println!(
        "argo forge promote: {} → version {} is now active (skill_id {})",
        skill_name, version, promoted.id.0
    );
    Ok(())
}

/// `argo forge runs count [--status] [--json]` — print just the
/// count of recorded forge runs. Caps the fetch at 10,000 rows
/// for safety.
async fn count_runs(
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
async fn list_runs(
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

