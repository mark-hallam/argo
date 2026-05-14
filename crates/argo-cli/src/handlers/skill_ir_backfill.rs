//! `argo skill ir backfill` — synthesise a minimal canonical IR
//! ([`SkillGraph`]) for every installed skill that lacks one (Track L
//! brief `L3-skill-ir-backfill`).
//!
//! The command walks `SkillStore::list_active`, hands the rows to
//! [`argo_skill::ir::backfill`], and prints either a one-line summary
//! (`N backfilled, M skipped`) or a JSON envelope under `--json`. The
//! `--dry-run` flag classifies every skill without writing to
//! `skill_ir`.
//!
//! Tenant scope: the local skill store has no tenant column, so the
//! backfill writes under the nil tenant (`TenantId(Uuid::nil())`) —
//! the same default the rest of the local CLI uses for tenant-scoped
//! primitives. Multi-tenant backfills run elsewhere (the publisher /
//! marketplace path that has tenants in scope).
use argo_skill::SkillStore;
use argo_skill::ir::backfill::backfill;
use argo_skill::ir::{
    BackfillEntry, BackfillReport, BackfillStatus, SqliteSkillIrStore, ensure_schema,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::sync::Arc;
use uuid::Uuid;

/// Entrypoint dispatched from [`crate::handlers::skill::run`].
pub async fn run(
    skills: &SkillStore,
    store: Arc<SessionStore>,
    dry_run: bool,
    json: bool,
) -> ArgoResult<()> {
    ensure_schema(&store).await?;
    let ir = SqliteSkillIrStore::new(&store);
    let rows = skills.list_active().await?;
    let tenant = TenantId(Uuid::nil());
    let report = backfill(&rows, &ir, tenant, dry_run).await?;
    if json {
        print_json(&report, dry_run)?;
    } else {
        print_summary(&report, dry_run);
    }
    Ok(())
}

fn print_summary(report: &BackfillReport, dry_run: bool) {
    let label = if dry_run { " (dry-run)" } else { "" };
    println!(
        "argo skill ir backfill{label}: {} skills backfilled, {} skipped (already have IR)",
        report.backfilled, report.skipped
    );
}

fn print_json(report: &BackfillReport, dry_run: bool) -> ArgoResult<()> {
    let entries: Vec<_> = report
        .entries
        .iter()
        .map(entry_to_json)
        .collect();
    let payload = serde_json::json!({
        "dry_run": dry_run,
        "backfilled": report.backfilled,
        "skipped": report.skipped,
        "entries": entries,
    });
    let serialized = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Provider(format!("skill ir backfill: encode json: {e}")))?;
    println!("{serialized}");
    Ok(())
}

fn entry_to_json(entry: &BackfillEntry) -> serde_json::Value {
    serde_json::json!({
        "skill_id": entry.skill_id.to_string(),
        "name": entry.name,
        "version": entry.version,
        "status": status_label(&entry.status),
    })
}

fn status_label(status: &BackfillStatus) -> &'static str {
    match status {
        BackfillStatus::Backfilled => "backfilled",
        BackfillStatus::Skipped => "skipped",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_label_matches_documented_wire_values() {
        assert_eq!(status_label(&BackfillStatus::Backfilled), "backfilled");
        assert_eq!(status_label(&BackfillStatus::Skipped), "skipped");
    }
}
