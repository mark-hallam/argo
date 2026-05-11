//! `argo persona history <worker_id>` — list append-only persona
//! revisions for one worker (Stream
//! `mission-persona-evolution-audit-revert`). Default output is a
//! human-readable table; `--json` emits a JSON array suitable for
//! piping into `jq` or `argo`'s test harness.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::persona_revision::{list_persona_revisions, PersonaRevision};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use uuid::Uuid;

/// Entry point for the `History` clap variant.
pub async fn run(worker_id: &str, limit: u32, json: bool) -> ArgoResult<()> {
    let worker_uuid = Uuid::parse_str(worker_id)
        .map_err(|e| ArgoError::Provider(format!("invalid worker id '{worker_id}': {e}")))?;
    let store = open_store().await?;
    let revisions = list_persona_revisions(store.pool(), worker_uuid, limit).await?;
    if json {
        print_json(&revisions);
    } else {
        print_table(worker_id, &revisions);
    }
    Ok(())
}

fn print_json(revisions: &[PersonaRevision]) {
    let payload = serde_json::to_string_pretty(revisions).unwrap_or_else(|_| "[]".into());
    println!("{payload}");
}

fn print_table(worker_id: &str, revisions: &[PersonaRevision]) {
    if revisions.is_empty() {
        println!("no persona revisions found for worker {worker_id}");
        return;
    }
    println!(
        "{:<36}  {:<20}  {:<16}  {:<24}  hash",
        "rev_id", "revised_at", "field", "actor"
    );
    for r in revisions {
        let revised_at = r.revised_at.to_rfc3339();
        let actor = truncate(&r.actor, 24);
        let hash = &r.audit_chain_hash[..12.min(r.audit_chain_hash.len())];
        println!(
            "{:<36}  {revised_at:<20}  {:<16}  {actor:<24}  {hash}…",
            r.rev_id,
            truncate(&r.field, 16)
        );
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}
