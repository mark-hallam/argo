use crate::cli::SessionCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId as StoreSessionId};

pub use super::session_orchestrator::run_handoff;

pub async fn run(cmd: SessionCmd) -> ArgoResult<()> {
    match cmd {
        SessionCmd::Start { tenant, title, worker, json } => super::session_orchestrator::start_orchestrator_session(tenant.as_deref(), &title, worker.as_deref(), json).await,
        SessionCmd::List {
            tenant,
            limit,
            json,
            format,
            since_minutes,
            older_than_minutes,
            spent_min,
            spent_max,
            provider,
            model,
            status,
            title_contains,
            sort_by,
            vertical,
        } => {
            if tenant.is_some() || std::env::var_os("ARGO_DEFAULT_TENANT").is_some() {
                return super::session_orchestrator::list_orchestrator_sessions(tenant.as_deref(), &status, limit, json).await;
            }
            list_sessions(
                limit,
                json,
                &format,
                since_minutes,
                older_than_minutes,
                spent_min,
                spent_max,
                provider,
                model,
                status,
                title_contains,
                sort_by,
                vertical,
            )
            .await
        }
        SessionCmd::Show {
            id,
            tenant,
            json,
            format,
        } => {
            if tenant.is_some() || std::env::var_os("ARGO_DEFAULT_TENANT").is_some() {
                return super::session_orchestrator::show_orchestrator_session(tenant.as_deref(), &id, json).await;
            }
            super::session_show::show_session(&id, json, &format).await
        }
        SessionCmd::Transcript { id, json } => {
            super::session_transcript::run(&id, json).await
        }
        SessionCmd::Count {
            status,
            since_minutes,
            older_than_minutes,
            provider,
            model,
            title_contains,
            spent_min,
            spent_max,
            json,
        } => {
            count_sessions(
                status,
                since_minutes,
                older_than_minutes,
                provider,
                model,
                title_contains,
                spent_min,
                spent_max,
                json,
            )
            .await
        }
        SessionCmd::Watch { id } => super::session_watch::run(&id).await,
        SessionCmd::Messages {
            id,
            json,
            format,
            limit,
            tail,
            role,
            since_minutes,
            seq_min,
            seq_max,
            content_contains,
        } => {
            super::session_messages::run(
                &id,
                json,
                &format,
                limit,
                tail,
                role.as_deref(),
                since_minutes,
                seq_min,
                seq_max,
                content_contains.as_deref(),
            )
            .await
        }
        SessionCmd::AttachSkill { id, skill } => {
            super::session_skills::attach(&id, &skill).await
        }
        SessionCmd::DetachSkill { id, skill } => {
            super::session_skills::detach(&id, &skill).await
        }
        SessionCmd::Skills { id } => super::session_skills::list(&id).await,
        SessionCmd::ClearSkills { id } => super::session_skills::clear(&id).await,
        SessionCmd::Export {
            id,
            out,
            include_audit,
            format,
        } => super::session_export::export_session(
            &id,
            out.as_deref(),
            include_audit,
            &format,
        )
        .await,
        SessionCmd::Search {
            query,
            limit,
            session,
            role,
            json,
            format,
            count,
        } => {
            super::session_search::run(
                &query,
                limit,
                session.as_deref(),
                role.as_deref(),
                json,
                &format,
                count,
            )
            .await
        }
        SessionCmd::Delete { id, force } => delete_session(&id, force).await,
        SessionCmd::Diff {
            id1,
            id2,
            json,
            format,
        } => diff_sessions(&id1, &id2, json, &format).await,
        SessionCmd::Bisect { id, hypothesis } => {
            super::session_bisect::run(&id, &hypothesis).await
        }
        SessionCmd::Title { id, new, clear } => {
            set_title(&id, new.as_deref(), clear).await
        }
        SessionCmd::End { id } => end_session(&id).await,
        SessionCmd::EndAll { older_than_minutes } => end_all(older_than_minutes).await,
        SessionCmd::Reopen { id } => reopen_session(&id).await,
        SessionCmd::Replay {
            id,
            model,
            provider,
            tools: _,
            workspace: _,
            stream,
        } => {
            super::session_replay::run(&id, model.as_deref(), provider.as_deref(), stream)
                .await
        }
        SessionCmd::Clone { id } => {
            super::session_clone_branch::clone_session(&id).await
        }
        SessionCmd::Branch { id, through_seq } => {
            super::session_clone_branch::branch_session(&id, through_seq).await
        }
        SessionCmd::Stats { id, json } => {
            super::session_clone_branch::stats_session(&id, json).await
        }
        SessionCmd::Turns { id, json } => {
            super::session_turns::run(&id, json).await
        }
        SessionCmd::ByDay { days, json } => {
            super::session_by_day::run(days, json).await
        }
        SessionCmd::ByProvider { since_minutes, json } => {
            super::session_by_provider::run(since_minutes, json).await
        }
        SessionCmd::ByModel { since_minutes, json } => {
            super::session_by_model::run(since_minutes, json).await
        }
        SessionCmd::AuditByKind {
            id,
            since_minutes,
            json,
        } => super::session_audit_rollups::run_by_kind(id, since_minutes, json).await,
        SessionCmd::AuditByTool {
            id,
            since_minutes,
            json,
        } => super::session_audit_rollups::run_by_tool(id, since_minutes, json).await,
        SessionCmd::MessagesByRole {
            id,
            since_minutes,
            json,
        } => super::session_messages_rollups::run_by_role(id, since_minutes, json).await,
        SessionCmd::ToolErrors {
            id,
            since_minutes,
            json,
        } => super::session_audit_rollups::run_tool_errors(id, since_minutes, json).await,
        SessionCmd::Approve {
            session_id,
            tool,
            note,
            once,
            json,
        } => {
            super::session_approval::approve(
                &session_id,
                &tool,
                note.as_deref(),
                once,
                json,
            )
            .await
        }
        SessionCmd::RevokeApproval {
            session_id,
            tool,
            json,
        } => super::session_approval::revoke_approval(&session_id, &tool, json).await,
        SessionCmd::ListGrants { session_id, json } => {
            super::session_approval::list_grants(&session_id, json).await
        }
        SessionCmd::CapabilityGaps {
            session_id,
            since_minutes,
            threshold,
            json,
        } => {
            super::session_capability_gaps::run(
                &session_id,
                since_minutes,
                threshold,
                json,
            )
            .await
        }
    }
}

use super::session_diff::diff_sessions;
use super::session_list::{count_sessions, list_sessions};

async fn reopen_session(raw: &str) -> ArgoResult<()> {
    let store = legacy_store().await?;
    let n = store.reopen_session(store_id(raw)?).await?;
    println!("{}", if n == 0 { format!("(session {raw} not found)") } else { format!("reopened {raw}") });
    Ok(())
}

async fn set_title(raw: &str, new: Option<&str>, clear: bool) -> ArgoResult<()> {
    let store = legacy_store().await?;
    let title = if clear { "" } else { new.unwrap_or("") };
    let n = store.set_session_title(store_id(raw)?, title).await?;
    println!("{}", if n == 0 { format!("(session {raw} not found)") } else if clear || title.trim().is_empty() { format!("cleared title on {raw}") } else { format!("set title on {raw} to {title:?}") });
    Ok(())
}

async fn end_session(raw: &str) -> ArgoResult<()> {
    let store = legacy_store().await?;
    let n = store.end_session(store_id(raw)?).await?;
    println!("{}", if n == 0 { format!("(session {raw} already ended or not found)") } else { format!("ended {raw}") });
    Ok(())
}

async fn end_all(older_than_minutes: Option<i64>) -> ArgoResult<()> {
    let store = legacy_store().await?;
    let rows = store.list_all_sessions(10_000).await?;
    let cutoff = older_than_minutes
        .map(|m| chrono::Utc::now() - chrono::Duration::minutes(m));
    let mut ended: u64 = 0;
    for s in rows
        .into_iter()
        .filter(|s| s.ended_at.is_none())
        .filter(|s| match cutoff {
            Some(c) => s.started_at < c,
            None => true,
        })
    {
        if let Ok(n) = store.end_session(StoreSessionId(s.id.0)).await {
            ended += n;
        }
    }
    println!("ended {ended} session(s)");
    Ok(())
}

async fn delete_session(raw: &str, force: bool) -> ArgoResult<()> {
    let store = legacy_store().await?;
    let id = store_id(raw)?;
    if !force {
        let header = store
            .get_session(id)
            .await?
            .ok_or_else(|| ArgoError::Provider(format!("no session {raw} (already deleted?)")))?;
        return Err(ArgoError::Provider(format!(
            "would delete session '{}' ({}/{}) — re-run with --force",
            header.title.as_deref().unwrap_or("(untitled)"),
            header.provider,
            header.model
        )));
    }
    let n = store.delete_session(id).await?;
    println!("{}", if n == 0 { format!("(no session {raw} - nothing deleted)") } else { format!("deleted {raw} ({n} session row + cascaded children)") });
    Ok(())
}

async fn legacy_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

fn store_id(raw: &str) -> ArgoResult<StoreSessionId> {
    uuid::Uuid::parse_str(raw).map(StoreSessionId).map_err(|e| ArgoError::Schema(format!("invalid session id '{raw}': {e}")))
}
