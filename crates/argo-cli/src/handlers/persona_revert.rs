//! `argo persona revert <rev_id>` — apply a prior revision's
//! `old_value` to the live persona, recording the revert itself as
//! a NEW append-only revision (Stream
//! `mission-persona-evolution-audit-revert`).
//!
//! The flow prints a three-line diff (target old / current / target
//! old applied) before prompting the operator, so they can see what
//! is about to overwrite the live state. `--yes` skips the prompt
//! for scripted use.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::persona_revision::{revert_persona_to, PersonaRevision};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use sqlx::Row;
use std::io::{self, Write};
use uuid::Uuid;

/// Entry point for the `Revert` clap variant.
pub async fn run(rev_id: &str, yes: bool, json: bool) -> ArgoResult<()> {
    let rev_uuid = Uuid::parse_str(rev_id)
        .map_err(|e| ArgoError::Schema(format!("invalid rev_id '{rev_id}': {e}")))?;
    let store = open_store().await?;
    let target = fetch_target(&store, rev_uuid).await?;
    let current = fetch_current_value(&store, &target).await?;

    if !json {
        print_diff(&target, &current);
    }
    if !yes && !confirm()? {
        println!("aborted; no changes made.");
        return Ok(());
    }

    let actor = resolve_actor();
    let new_rev = revert_persona_to(store.pool(), rev_uuid, &actor).await?;
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "reverted_rev_id": target.rev_id,
            "new_rev_id": new_rev.rev_id,
            "field": new_rev.field,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!(
            "reverted persona {} field={} (new rev_id={})",
            target.worker_id, target.field, new_rev.rev_id
        );
    }
    Ok(())
}

async fn fetch_target(store: &SessionStore, rev_id: Uuid) -> ArgoResult<PersonaRevision> {
    let row = sqlx::query(
        "SELECT rev_id, worker_id, revised_at, field, old_value, new_value, actor, \
         audit_chain_hash FROM persona_revision WHERE rev_id = ?",
    )
    .bind(rev_id.to_string())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("revert target lookup: {e}")))?
    .ok_or_else(|| ArgoError::Provider(format!("revision {rev_id} not found")))?;
    Ok(PersonaRevision {
        rev_id: parse_uuid_col(&row, "rev_id")?,
        worker_id: parse_uuid_col(&row, "worker_id")?,
        revised_at: chrono::DateTime::from_timestamp(row.get::<i64, _>("revised_at"), 0)
            .ok_or_else(|| ArgoError::Provider("invalid revised_at".into()))?,
        field: row.get("field"),
        old_value: row.get("old_value"),
        new_value: row.get("new_value"),
        actor: row.get("actor"),
        audit_chain_hash: row.get("audit_chain_hash"),
    })
}

fn parse_uuid_col(row: &sqlx::sqlite::SqliteRow, col: &str) -> ArgoResult<Uuid> {
    let raw: String = row.get(col);
    Uuid::parse_str(&raw).map_err(|e| ArgoError::Provider(format!("parse {col} '{raw}': {e}")))
}

async fn fetch_current_value(store: &SessionStore, target: &PersonaRevision) -> ArgoResult<String> {
    let column = persona_column(&target.field)?;
    let value: Option<String> =
        sqlx::query_scalar(&format!("SELECT {column} FROM persona WHERE worker_id = ?"))
            .bind(target.worker_id.to_string())
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Io(format!("read current persona value: {e}")))?
            .ok_or_else(|| {
                ArgoError::Provider(format!(
                    "persona row missing for worker_id {} — cannot revert",
                    target.worker_id
                ))
            })?;
    Ok(value.unwrap_or_default())
}

fn persona_column(field: &str) -> ArgoResult<&'static str> {
    match field {
        "name" => Ok("name"),
        "voice" | "voice_profile" | "voice_preamble" => Ok("voice_preamble"),
        "avatar" => Ok("avatar"),
        "bio" => Ok("bio"),
        other => Err(ArgoError::Schema(format!(
            "persona field '{other}' is not revertible (allowed: name, voice, avatar, bio)"
        ))),
    }
}

fn print_diff(target: &PersonaRevision, current: &str) {
    println!("revision {} (worker {})", target.rev_id, target.worker_id);
    println!("  field:    {}", target.field);
    println!("  recorded at: {}", target.revised_at.to_rfc3339());
    println!("  recorded actor: {}", target.actor);
    println!("  diff (target -> revert):");
    println!("    target.old_value (will be applied) : {}", target.old_value);
    println!("    current live value (will be lost)  : {current}");
    println!("    target.new_value (for reference)   : {}", target.new_value);
}

fn confirm() -> ArgoResult<bool> {
    print!("apply revert? [y/N]: ");
    io::stdout()
        .flush()
        .map_err(|e| ArgoError::Io(format!("stdout flush: {e}")))?;
    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .map_err(|e| ArgoError::Io(format!("stdin read: {e}")))?;
    let trimmed = buf.trim().to_lowercase();
    Ok(matches!(trimmed.as_str(), "y" | "yes"))
}

fn resolve_actor() -> String {
    let user = std::env::var("USER").unwrap_or_else(|_| "unknown".into());
    format!("cli:{user}")
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}
