//! `argo judge` — verdict inspection. Override / re-run live with
//! the daemon's judge worker (separate PR).
use crate::cli::JudgeCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::{DateTime, Utc};

pub async fn run(cmd: JudgeCmd) -> ArgoResult<()> {
    match cmd {
        JudgeCmd::Verdict { id } => verdict(&id).await,
        JudgeCmd::Override { id, to } => {
            println!(
                "argo judge override {id} → {to}: requires the daemon's judge worker (separate PR)."
            );
            Ok(())
        }
    }
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
