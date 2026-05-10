//! `argo judge` — verdict inspection + offline override.
//!
//! `verdict <session-id>` lists the persisted verdicts on a
//! session. `override <record-id> --to <verdict>` updates a
//! single judge_record in place + writes a `JudgeVerdict` audit
//! row capturing the change. Re-running a verdict against a live
//! judge stays with the daemon's worker.
use crate::cli::JudgeCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, JudgeRecordId, SessionId};
use chrono::{DateTime, Utc};
use std::sync::Arc;

pub async fn run(cmd: JudgeCmd) -> ArgoResult<()> {
    match cmd {
        JudgeCmd::Verdict { id } => verdict(&id).await,
        JudgeCmd::Override { id, to } => override_verdict(&id, &to).await,
    }
}

async fn override_verdict(raw: &str, new_verdict: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let record_id = JudgeRecordId(uuid::Uuid::parse_str(raw).map_err(|e| {
        ArgoError::Provider(format!("invalid judge record id '{raw}': {e}"))
    })?);
    let trimmed = new_verdict.trim();
    if trimmed.is_empty() {
        return Err(ArgoError::Provider("empty --to verdict".into()));
    }
    let reason = format!("offline override via `argo judge override` (record {raw})");
    let outcome = store
        .override_judge_verdict(record_id, trimmed, Some(&reason))
        .await?;
    let Some((session_id, old)) = outcome else {
        return Err(ArgoError::Provider(format!(
            "no judge record with id {raw}"
        )));
    };
    AuditSink::new(Arc::clone(&store))
        .write(
            Some(session_id),
            AuditKind::JudgeVerdict,
            serde_json::json!({
                "kind": "override",
                "record_id": record_id.0.to_string(),
                "old_verdict": old,
                "new_verdict": trimmed,
                "by": "cli",
            }),
            None,
        )
        .await?;
    println!(
        "argo judge override: {raw} {old} → {trimmed} (session {})",
        session_id.0
    );
    Ok(())
}

async fn verdict(raw: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    let rows = store.list_judge_records(SessionId(id)).await?;
    if rows.is_empty() {
        println!("(no judge verdicts for session {raw})");
        return Ok(());
    }
    println!("{} verdict(s):", rows.len());
    for r in rows {
        let dt = DateTime::<Utc>::from_timestamp_micros(r.created_at).unwrap_or_else(Utc::now);
        let provider_model = format!("{}:{}", r.judge_provider, r.judge_model);
        let rubric = format!("{}@{}", r.rubric_id, r.rubric_version);
        println!(
            "  {dt}  {kind:<10} {subject:<24} {verdict:<8} {provider_model:<30} {rubric:<14} {reason}",
            kind = r.subject_kind,
            subject = r.subject_ref,
            verdict = r.verdict,
            reason = r.reason.unwrap_or_default()
        );
    }
    Ok(())
}
