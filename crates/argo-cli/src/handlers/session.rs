use crate::cli::{HandoffCmd, SessionCmd};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::WorkerId;
use argo_orchestrator::{EscalationStep, Handoff, HandoffId, HandoffStatus, OrchestratorStore, Session};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId as StoreSessionId, TenantId};
use std::str::FromStr;
use std::sync::Arc;
pub async fn run(cmd: SessionCmd) -> ArgoResult<()> {
    match cmd {
        SessionCmd::Start { tenant, title, json } => start_orchestrator_session(tenant.as_deref(), &title, json).await,
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
                return list_orchestrator_sessions(tenant.as_deref(), &status, limit, json).await;
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
                return show_orchestrator_session(tenant.as_deref(), &id, json).await;
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
pub async fn run_handoff(cmd: HandoffCmd) -> ArgoResult<()> {
    match cmd {
        HandoffCmd::List { tenant, session, status, worker, limit, json } => list_handoffs(tenant.as_deref(), session.as_deref(), status.as_deref(), worker.as_deref(), limit, json).await,
        HandoffCmd::Show { id, tenant, json } => show_handoff(tenant.as_deref(), &id, json).await,
    }
}
async fn start_orchestrator_session(
    tenant: Option<&str>,
    title: &str,
    json: bool,
) -> ArgoResult<()> {
    let tenant = tenant_id(tenant)?;
    let repo = orchestrator_repo().await?;
    let session = Session::new(tenant, title);
    repo.insert_session(&session).await?;
    print_json_or_text(json, &session, || session.id.to_string())
}
async fn list_orchestrator_sessions(tenant: Option<&str>, status: &str, limit: u32, json: bool) -> ArgoResult<()> {
    let tenant = tenant_id(tenant)?;
    let repo = orchestrator_repo().await?;
    if !matches!(status, "all" | "active") {
        return Err(ArgoError::Provider("current persist API can list only active orchestrator sessions".into()));
    }
    let mut out = repo.list_active_sessions(tenant).await?;
    out.truncate(usize::try_from(limit.clamp(1, 500)).unwrap_or(25));
    if json {
        println!("{}", serde_json::to_string(&out).unwrap_or_default())
    } else {
        for row in &out {
            println!("{}\t{:?}\t{}", row.id, row.status, row.title);
        }
    }
    Ok(())
}
async fn show_orchestrator_session(tenant: Option<&str>, id: &str, json: bool) -> ArgoResult<()> {
    let tenant = tenant_id(tenant)?;
    let repo = orchestrator_repo().await?;
    let sid = argo_orchestrator::SessionId::from_string(id.to_string());
    let Some(session) = repo.find_session(tenant, &sid).await? else {
        return Err(ArgoError::Provider(format!("no orchestrator session {id}")));
    };
    let handoffs = repo.list_handoffs_for_session(tenant, &sid).await?;
    let pending: Vec<_> = handoffs
        .into_iter()
        .filter(|h| h.status == HandoffStatus::Pending)
        .map(|h| handoff_json(&h))
        .collect();
    let body = serde_json::json!({"session":session,"participants":session.participants,"pending_handoffs":pending});
    if json {
        println!("{}", serde_json::to_string(&body).unwrap_or_default());
    } else {
        println!(
            "{}\t{:?}\t{}\t{} pending",
            session.id,
            session.status,
            session.title,
            body["pending_handoffs"].as_array().map_or(0, Vec::len)
        );
    }
    Ok(())
}
async fn list_handoffs(
    tenant: Option<&str>,
    session: Option<&str>,
    status: Option<&str>,
    worker: Option<&str>,
    limit: u32,
    json: bool,
) -> ArgoResult<()> {
    let tenant = tenant_id(tenant)?;
    let repo = orchestrator_repo().await?;
    let wanted = status.map(parse_handoff_status).transpose()?;
    let mut out = if let Some(s) = session {
        repo.list_handoffs_for_session(tenant, &argo_orchestrator::SessionId::from_string(s.to_string())).await?
    } else if let Some(w) = worker {
        repo.list_pending_handoffs_for_worker(tenant, WorkerId::from_str(w).map_err(|e| ArgoError::Provider(format!("invalid worker id '{w}': {e}")))?).await?
    } else {
        return Err(ArgoError::Provider("handoff list requires --session or --worker".into()));
    };
    if let Some(w) = worker {
        out.retain(|h| worker_matches(h, w));
    }
    if let Some(s) = wanted {
        out.retain(|h| h.status == s);
    }
    out.truncate(usize::try_from(limit.clamp(1, 500)).unwrap_or(25));
    if json {
        let arr: Vec<_> = out.iter().map(handoff_json).collect();
        println!("{}", serde_json::to_string(&arr).unwrap_or_default());
    } else {
        for row in &out {
            println!("{}\t{:?}\t{}", row.id, row.status, row.expected_outcome);
        }
    }
    Ok(())
}
async fn show_handoff(tenant: Option<&str>, id: &str, json: bool) -> ArgoResult<()> {
    let tenant = tenant_id(tenant)?;
    let repo = orchestrator_repo().await?;
    let hid = HandoffId::from_string(id.to_string());
    let Some(handoff) = repo.find_handoff(tenant, &hid).await? else {
        return Err(ArgoError::Provider(format!("no handoff {id}")));
    };
    if json {
        println!("{}", serde_json::to_string(&handoff_json(&handoff)).unwrap_or_default());
    } else {
        println!("{}\t{:?}\t{}", handoff.id, handoff.status, handoff.expected_outcome);
    }
    Ok(())
}
async fn orchestrator_repo() -> ArgoResult<OrchestratorStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let repo = OrchestratorStore::new(store);
    repo.ensure_schema().await?;
    Ok(repo)
}
fn tenant_id(raw: Option<&str>) -> ArgoResult<TenantId> {
    let raw = raw.map(ToOwned::to_owned).or_else(|| std::env::var("ARGO_DEFAULT_TENANT").ok())
        .ok_or_else(|| ArgoError::Provider("missing --tenant (or ARGO_DEFAULT_TENANT)".into()))?;
    uuid::Uuid::parse_str(&raw).map(TenantId).map_err(|e| ArgoError::Provider(format!("invalid tenant id '{raw}': {e}")))
}
fn parse_handoff_status(raw: &str) -> ArgoResult<HandoffStatus> {
    match raw {
        "pending" => Ok(HandoffStatus::Pending),
        "acknowledged" => Ok(HandoffStatus::Acknowledged),
        "settled" => Ok(HandoffStatus::Settled),
        "expired" => Ok(HandoffStatus::Expired),
        other => Err(ArgoError::Provider(format!("invalid handoff status '{other}'"))),
    }
}
fn worker_matches(h: &Handoff, raw: &str) -> bool {
    h.from.to_string() == raw || matches!(&h.to, EscalationStep::Worker(id) if id.to_string() == raw)
}
fn handoff_json(h: &Handoff) -> serde_json::Value {
    let to = match &h.to {
        EscalationStep::Worker(id) => serde_json::json!({"kind":"worker","worker_id":id.to_string()}),
        EscalationStep::Human(addr) => serde_json::json!({"kind":"human","addr":addr}),
    };
    serde_json::json!({"id":h.id.to_string(),"session_id":h.session_id.to_string(),"from":h.from.to_string(),"to":to,"expected_outcome":h.expected_outcome,"payload":h.payload,"status":handoff_status_label(h.status),"created_at":h.created_at.to_rfc3339(),"deadline":h.deadline.map(|d| d.to_rfc3339())})
}
fn handoff_status_label(s: HandoffStatus) -> &'static str {
    match s { HandoffStatus::Pending => "pending", HandoffStatus::Acknowledged => "acknowledged", HandoffStatus::Settled => "settled", HandoffStatus::Expired => "expired" }
}
fn print_json_or_text<T: serde::Serialize>(json: bool, value: &T, text: impl FnOnce() -> String) -> ArgoResult<()> {
    if json { println!("{}", serde_json::to_string(value).unwrap_or_default()); } else { println!("{}", text()); }
    Ok(())
}
use super::session_diff::diff_sessions;
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
    uuid::Uuid::parse_str(raw).map(StoreSessionId).map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))
}
use super::session_list::{count_sessions, list_sessions};
#[cfg(test)]
mod orchestrator_cli_tests {
    use super::*;
    #[tokio::test]
    async fn in_memory_handoff_json_shape_round_trips() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap(); let repo = OrchestratorStore::new(store); repo.ensure_schema().await.unwrap();
        let tenant = TenantId::new();
        let session = Session::new(tenant, "demo");
        repo.insert_session(&session).await.unwrap();
        let h = Handoff::new(session.id.clone(), WorkerId::new(), EscalationStep::Worker(WorkerId::new()), "finish demo", serde_json::json!({"case":"demo"}));
        repo.insert_handoff(tenant, &h).await.unwrap();
        let rows = repo.list_handoffs_for_session(tenant, &session.id).await.unwrap(); let v = handoff_json(&rows[0]);
        assert_eq!(v["status"], "pending"); assert_eq!(v["to"]["kind"], "worker"); assert_eq!(v["payload"]["case"], "demo");
    }
}
