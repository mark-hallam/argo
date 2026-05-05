//! `argo worker doctor` — one-shot SKYCOT health check (#739).
//!
//! Surveys the worker setup so operators can answer
//! "is this healthy?" without scripting 5 commands together.
//! Reports:
//!
//! * Loaded verticals + how many declare an `[actions]` section
//!   (no [actions] = no Worker v3 emission allowed for that
//!   vertical — the OS treats every action as `Forbidden`).
//! * Action-ledger counts grouped by state (pending,
//!   approved, executing, completed, failed, blocked) split by
//!   approval class so operators see "what needs review?" at a
//!   glance.
//! * Recent failed + blocked rows (last 24h, capped at 10 each)
//!   with the failure reason / blocked_reason inline.
//! * Workspace ledger file mtimes — when did
//!   `.skycot/{approval_queue,completed_actions,blocked_actions}.md`
//!   last get refreshed?
//!
//! Pure read-only; never mutates anything. Failure modes log
//! to stderr + continue so a partial report is more useful
//! than no report.

use super::worker_actions::ActionRecord;
use super::worker_actions_transitions;
use argo_memory::{MemoryFile, MemoryStore};
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;

const RECENT_FAIL_CAP: usize = 10;
const RECENT_WINDOW_HOURS: i64 = 24;

#[derive(Debug, Serialize)]
struct VerticalReport {
    id: String,
    actions_declared: usize,
}

#[derive(Debug, Serialize, Default)]
struct LedgerCounts {
    pending_autonomous: usize,
    pending_approval: usize,
    approved: usize,
    executing: usize,
    completed: usize,
    failed: usize,
    blocked: usize,
    total: usize,
}

#[derive(Debug, Serialize)]
struct RecentRow {
    action_id: String,
    action_type: String,
    job_id: String,
    state: String,
    reason: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct LedgerFileStat {
    name: String,
    exists: bool,
    last_refresh: Option<String>,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    workspace_dir: String,
    verticals: Vec<VerticalReport>,
    counts: LedgerCounts,
    recent_failed: Vec<RecentRow>,
    recent_blocked: Vec<RecentRow>,
    ledger_files: Vec<LedgerFileStat>,
}

pub(crate) async fn run(vertical: Option<&str>, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let report = build_report(&memory, ws, &cwd, vertical).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into())
        );
    } else {
        print_text(&report);
    }
    Ok(())
}

async fn build_report(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    cwd: &std::path::Path,
    vertical_filter: Option<&str>,
) -> ArgoResult<DoctorReport> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let verticals: Vec<VerticalReport> = registry
        .list()
        .filter(|(id, _)| vertical_filter.is_none_or(|v| *id == v))
        .map(|(id, loaded)| VerticalReport {
            id: id.to_string(),
            actions_declared: loaded
                .manifest
                .actions
                .as_ref()
                .map(|a| a.actions.len())
                .unwrap_or(0),
        })
        .collect();
    let mut counts = LedgerCounts::default();
    let mut recent_failed: Vec<RecentRow> = Vec::new();
    let mut recent_blocked: Vec<RecentRow> = Vec::new();
    let cutoff = Utc::now() - chrono::Duration::hours(RECENT_WINDOW_HOURS);
    let records = collect_latest(memory, workspace, vertical_filter, &registry).await;
    for r in &records {
        match (r.state.as_str(), r.requires_approval) {
            ("pending", true) => counts.pending_approval += 1,
            ("pending", false) => counts.pending_autonomous += 1,
            ("approved", _) => counts.approved += 1,
            ("executing", _) => counts.executing += 1,
            ("completed", _) => counts.completed += 1,
            ("failed", _) => counts.failed += 1,
            ("blocked", _) => counts.blocked += 1,
            _ => {}
        }
        counts.total += 1;
        if r.state == "failed" || r.state == "blocked" {
            let row = RecentRow {
                action_id: r.action_id.clone(),
                action_type: r.action_type.clone(),
                job_id: r.job_id.clone(),
                state: r.state.clone(),
                reason: r
                    .blocked_reason
                    .clone()
                    .or_else(|| {
                        r.result
                            .as_ref()
                            .and_then(|v| v.get("failure"))
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    })
                    .unwrap_or_default(),
                created_at: r.created_at.clone(),
            };
            if let Ok(ts) = DateTime::parse_from_rfc3339(&r.created_at) {
                if ts.with_timezone(&Utc) >= cutoff {
                    if r.state == "failed" {
                        recent_failed.push(row);
                    } else {
                        recent_blocked.push(row);
                    }
                }
            }
        }
    }
    recent_failed.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    recent_failed.truncate(RECENT_FAIL_CAP);
    recent_blocked.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    recent_blocked.truncate(RECENT_FAIL_CAP);

    let skycot_dir = cwd.join(".skycot");
    let ledger_files: Vec<LedgerFileStat> = [
        "approval_queue.md",
        "completed_actions.md",
        "blocked_actions.md",
    ]
    .iter()
    .map(|name| {
        let path = skycot_dir.join(name);
        let (exists, mtime) = match std::fs::metadata(&path) {
            Ok(meta) => {
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| DateTime::<Utc>::from(t).into())
                    .map(|t: DateTime<Utc>| t.to_rfc3339());
                (true, mtime)
            }
            Err(_) => (false, None),
        };
        LedgerFileStat {
            name: name.to_string(),
            exists,
            last_refresh: mtime,
        }
    })
    .collect();
    Ok(DoctorReport {
        workspace_dir: cwd.to_string_lossy().to_string(),
        verticals,
        counts,
        recent_failed,
        recent_blocked,
        ledger_files,
    })
}

async fn collect_latest(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    vertical_filter: Option<&str>,
    registry: &argo_vertical::Registry,
) -> Vec<ActionRecord> {
    let mut latest: BTreeMap<String, ActionRecord> = BTreeMap::new();
    for (vid, _loaded) in registry.list() {
        if vertical_filter.is_some_and(|v| vid != v) {
            continue;
        }
        let entries = match memory
            .list_for_vertical(workspace, MemoryFile::Memory, vid)
            .await
        {
            Ok(e) => e,
            Err(e) => {
                eprintln!("argo worker doctor: warning — vertical '{vid}' read failed: {e}");
                continue;
            }
        };
        for e in entries {
            let Ok(rec) = serde_json::from_str::<ActionRecord>(&e.body) else {
                continue;
            };
            if rec.schema_key != ActionRecord::SCHEMA_KEY {
                continue;
            }
            match latest.get(&rec.action_id) {
                Some(prev) if prev.created_at >= rec.created_at => {}
                _ => {
                    latest.insert(rec.action_id.clone(), rec);
                }
            }
        }
    }
    latest.into_values().collect()
}

fn print_text(r: &DoctorReport) {
    println!("SKYCOT health");
    println!("=============");
    println!("Workspace:        {}", r.workspace_dir);
    println!();
    println!("Verticals loaded ({}):", r.verticals.len());
    if r.verticals.is_empty() {
        println!("  (no verticals — set ARGO_VERTICALS_DIR or place under verticals/)");
    } else {
        for v in &r.verticals {
            println!("  - {} ({} action(s) declared)", v.id, v.actions_declared);
        }
    }
    println!();
    println!("Action ledger ({} total):", r.counts.total);
    println!("  pending (autonomous): {}", r.counts.pending_autonomous);
    println!("  pending (approval):   {}", r.counts.pending_approval);
    println!("  approved:             {}", r.counts.approved);
    println!("  executing:            {}", r.counts.executing);
    println!("  completed:            {}", r.counts.completed);
    println!("  failed:               {}", r.counts.failed);
    println!("  blocked:              {}", r.counts.blocked);
    println!();
    if !r.recent_failed.is_empty() {
        println!(
            "Recent failures (last {}h, top {}):",
            RECENT_WINDOW_HOURS, RECENT_FAIL_CAP
        );
        for row in &r.recent_failed {
            println!("  - {} {} — {}", row.action_id, row.action_type, row.reason);
        }
        println!();
    }
    if !r.recent_blocked.is_empty() {
        println!(
            "Recent blocked (last {}h, top {}):",
            RECENT_WINDOW_HOURS, RECENT_FAIL_CAP
        );
        for row in &r.recent_blocked {
            println!("  - {} {} — {}", row.action_id, row.action_type, row.reason);
        }
        println!();
    }
    println!("Workspace ledger files:");
    for f in &r.ledger_files {
        match (f.exists, f.last_refresh.as_deref()) {
            (true, Some(t)) => println!("  {:<24} last refresh {t}", f.name),
            (true, None) => println!("  {:<24} (mtime unavailable)", f.name),
            (false, _) => println!(
                "  {:<24} (not yet rendered — run `argo worker actions render`)",
                f.name
            ),
        }
    }
}

#[cfg(test)]
#[path = "worker_doctor_tests.rs"]
mod tests;
