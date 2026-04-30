//! `argo forge` — runs list / show / abort (offline ops only).
//! Optimize / promote / bench live with the daemon binding
//! (separate PR — they need a live provider + judge).
use crate::cli::{ForgeCmd, ForgeRunsCmd};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::{AuditKind, AuditSink};
use argo_eval::{load_jsonl, run_eval, EvalSubject, EvalTask};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;

pub async fn run(cmd: ForgeCmd) -> ArgoResult<()> {
    match cmd {
        ForgeCmd::Runs { cmd } => match cmd {
            ForgeRunsCmd::List => list_runs().await,
            ForgeRunsCmd::Show { id } => show_run(&id).await,
            ForgeRunsCmd::Abort { id } => abort_run(&id).await,
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
        } => optimize(&skill, &examples, iterations, budget).await,
    }
}

/// Best-of-N sweep over `forge bench`. Runs the bench
/// \[iterations\] times against the same examples, tracking the
/// score per iteration; the best score wins. Aborts early when
/// the per-session spend tally exceeds the `--budget` cap (in
/// USD). The full skill-mutation pipeline is daemon-side; this
/// CLI-driven loop is the consistency-check tier — useful for
/// surfacing flaky tasks + measuring run-to-run variance.
async fn optimize(
    skill_name: &str,
    examples_path: &str,
    iterations: u32,
    budget_usd: f64,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let store = Arc::new(open_store().await?);
    let skills = SkillStore::new(Arc::clone(&store));
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
        "argo forge optimize: {skill_name} v{} → {} iters × {} task(s); budget ${budget_usd:.2}",
        active.frontmatter.version,
        iterations,
        tasks.len()
    );

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
                "iterations": iterations,
                "budget_usd": budget_usd,
            }),
        )
        .await;

    let subject = BenchSubject {
        store: Arc::clone(&store),
        cwd: cwd.clone(),
        tenant,
        workspace,
        provider: "anthropic".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        skill_name: skill_name.to_string(),
    };

    let mut best: f64 = f64::NEG_INFINITY;
    let mut completed = 0_i64;
    let mut last_status = "succeeded";
    let budget_micros = (budget_usd * 1_000_000.0).round() as i64;
    let baseline_spend = store
        .get_forge_run(run_id)
        .await
        .ok()
        .flatten()
        .map_or(0_i64, |r| r.spend_usd_micros);
    let _ = baseline_spend; // not used yet — kept for the over-budget cutoff.

    println!("\n  iter  mean_score  pass_rate  errors  best_so_far");
    for iter in 0..iterations {
        let (records, summary) = run_eval(&subject, &tasks).await;
        if summary.errors > 0 {
            last_status = if summary.errors == summary.n {
                "failed"
            } else {
                "partial"
            };
        }
        if summary.mean_score > best {
            best = summary.mean_score;
        }
        completed += 1;
        println!(
            "  {iter:>3}   {mean:.3}      {pr:.3}     {errs:<6}  {best:.3}",
            iter = iter,
            mean = summary.mean_score,
            pr = summary.pass_rate,
            errs = summary.errors,
            best = best,
        );
        // Drop the records we don't need into oblivion.
        drop(records);

        // Cheap budget check: query the spend ledger snapshot.
        if let Ok(Some(row)) = store.get_forge_run(run_id).await {
            if budget_micros > 0 && row.spend_usd_micros >= budget_micros {
                last_status = "partial";
                println!(
                    "  (over budget after iter {iter}: spend=${:.2} > cap=${budget_usd:.2})",
                    row.spend_usd_micros as f64 / 1_000_000.0
                );
                break;
            }
        }
    }

    let final_score = if best.is_finite() { Some(best) } else { None };
    let _ = store
        .finish_forge_run(run_id, last_status, final_score, completed)
        .await;
    let _ = AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunEnd,
            serde_json::json!({
                "run_id": run_id.to_string(),
                "status": last_status,
                "best_score": final_score,
                "iterations_run": completed,
            }),
        )
        .await;
    println!(
        "\nsummary: status={last_status} best_score={best:.3} iters_run={completed}/{iterations} (run_id={run_id})"
    );
    Ok(())
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

/// `EvalSubject` impl that runs each task as a fresh recorded
/// session via `argo_agent_runner::run_turn`. Returns the final
/// assistant text; the runner's metric scores it.
#[allow(dead_code)]
struct BenchSubject {
    store: Arc<SessionStore>,
    cwd: PathBuf,
    tenant: argo_types::TenantId,
    workspace: argo_types::WorkspaceId,
    provider: String,
    model: String,
    skill_name: String,
}

#[async_trait]
impl EvalSubject for BenchSubject {
    async fn run(&self, task: &EvalTask) -> ArgoResult<String> {
        let title = format!("forge bench: {}", task.name);
        let sid = self
            .store
            .create_session(
                self.tenant,
                self.workspace,
                &self.model,
                &self.provider,
                Some(&title),
            )
            .await?;
        let result = argo_agent_runner::run_turn(
            &self.store,
            &self.cwd,
            SessionId(sid.0),
            Some(&task.input),
            false,
            argo_agent_runner::MAX_TOOL_ROUNDS,
        )
        .await?;
        Ok(result.text)
    }
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

async fn list_runs() -> ArgoResult<()> {
    let store = open_store().await?;
    let rows = store.list_forge_runs(50).await?;
    if rows.is_empty() {
        println!("(no forge runs recorded yet)");
        return Ok(());
    }
    println!(
        "id        skill                    base       candidate  iters  spend USD  score   status"
    );
    for r in rows {
        let cand = r.candidate_version.unwrap_or_else(|| "-".into());
        let score = r
            .final_score
            .map(|s| format!("{s:.3}"))
            .unwrap_or_else(|| "-".into());
        let spend = format!("${:.2}", r.spend_usd_micros as f64 / 1_000_000.0);
        let id_short: String = hex::encode(&r.id).chars().take(8).collect();
        println!(
            "{id_short:<8}  {name:<24} {base:<10} {cand:<10} {iters:<6} {spend:<10} {score:<7} {status}",
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

async fn show_run(raw: &str) -> ArgoResult<()> {
    let store = open_store().await?;
    let id = parse_run_id(raw)?;
    let row = store
        .get_forge_run(id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no forge run with id {raw}")))?;
    let started = ts(row.started_at);
    let ended = row
        .ended_at
        .map(ts)
        .unwrap_or_else(|| "(running)".to_string());
    println!("forge run {}", hex::encode(&row.id));
    println!("  skill:        {}", row.skill_name);
    println!("  status:       {}", row.status);
    println!("  base:         {}", row.base_version);
    println!(
        "  candidate:    {}",
        row.candidate_version.as_deref().unwrap_or("(none)")
    );
    println!("  iterations:   {}", row.iterations);
    println!(
        "  spend:        ${:.2}",
        row.spend_usd_micros as f64 / 1_000_000.0
    );
    println!(
        "  final_score:  {}",
        row.final_score
            .map(|s| format!("{s:.3}"))
            .unwrap_or_else(|| "-".into())
    );
    println!("  started_at:   {started}");
    println!("  ended_at:     {ended}");
    Ok(())
}

async fn abort_run(raw: &str) -> ArgoResult<()> {
    let store = Arc::new(open_store().await?);
    let id = parse_run_id(raw)?;
    let affected = store.abort_forge_run(id).await?;
    if affected == 0 {
        return Err(ArgoError::Provider(format!("no forge run with id {raw}")));
    }
    AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunEnd,
            serde_json::json!({
                "kind": "abort",
                "run_id": id.to_string(),
                "by": "cli",
            }),
        )
        .await?;
    println!("argo forge runs abort: {raw} → status=aborted");
    Ok(())
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

/// Resolve a forge-run id from the user's input. Accepts either a
/// full UUID or an 8-hex-char short prefix (matching what
/// `argo forge runs list` prints).
fn parse_run_id(raw: &str) -> ArgoResult<uuid::Uuid> {
    if let Ok(u) = uuid::Uuid::parse_str(raw) {
        return Ok(u);
    }
    if raw.len() >= 8 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ArgoError::Provider(format!(
            "short prefix '{raw}' — pass the full uuid (see `argo forge runs list`)"
        )));
    }
    Err(ArgoError::Provider(format!("invalid forge run id '{raw}'")))
}

fn ts(micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(micros)
        .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "?".into())
}
