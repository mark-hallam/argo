//! `argo forge` — runs list / show / abort (offline ops only).
//! Optimize / promote / bench live with the daemon binding
//! (separate PR — they need a live provider + judge).
use crate::cli::{ForgeCmd, ForgeRunsCmd};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::{AuditKind, AuditSink};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use std::sync::Arc;

pub async fn run(cmd: ForgeCmd) -> ArgoResult<()> {
    match cmd {
        ForgeCmd::Runs { cmd } => match cmd {
            ForgeRunsCmd::List => list_runs().await,
            ForgeRunsCmd::Show { id } => show_run(&id).await,
            ForgeRunsCmd::Abort { id } => abort_run(&id).await,
        },
        ForgeCmd::Promote { skill, version } => promote(&skill, &version).await,
        other => {
            println!("argo forge: '{other:?}' requires the daemon's forge worker (separate PR).");
            Ok(())
        }
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
