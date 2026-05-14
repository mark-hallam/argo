//! Orchestrator-mode session and handoff CLI subcommands.
//!
//! Split out of `handlers/session.rs` to keep that file under the
//! 500-LOC ceiling. This module owns multi-tenant orchestrator session
//! ops (`Start`/`List`/`Show` when `--tenant` or `ARGO_DEFAULT_TENANT`
//! is set) plus the entire `argo handoff …` subcommand surface.
use crate::cli::HandoffCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::WorkerId;
use argo_orchestrator::{EscalationStep, Handoff, HandoffId, HandoffStatus, OrchestratorStore, Session};
use argo_store::{SessionStore, WorkerId as StoreWorkerId};
use argo_types::{ArgoError, ArgoResult, SessionId as StoreSessionId, TenantId};
use std::str::FromStr;
use std::sync::Arc;

pub async fn run_handoff(cmd: HandoffCmd) -> ArgoResult<()> {
    match cmd {
        HandoffCmd::List { tenant, session, status, worker, limit, json } => list_handoffs(tenant.as_deref(), session.as_deref(), status.as_deref(), worker.as_deref(), limit, json).await,
        HandoffCmd::Show { id, tenant, json } => show_handoff(tenant.as_deref(), &id, json).await,
        HandoffCmd::Cancel { id, tenant, note, json } => cancel_handoff(tenant.as_deref(), &id, note.as_deref(), json).await,
    }
}

pub(super) async fn start_orchestrator_session(tenant: Option<&str>, title: &str, worker: Option<&str>, json: bool) -> ArgoResult<()> {
    let tenant = tenant_id(tenant)?;
    let worker = worker.map(WorkerId::from_str).transpose().map_err(|e| ArgoError::Schema(format!("invalid worker id: {e}")))?;
    let (store, repo) = orchestrator_store_and_repo().await?;
    let mut session = Session::new(tenant, title);
    if let Some(w) = worker { session.add_participant(w); }
    repo.insert_session(&session).await?;
    if let Some(w) = worker {
        let sid = uuid::Uuid::parse_str(session.id.as_str()).map(StoreSessionId).map_err(|e| ArgoError::Schema(format!("invalid session id '{}': {e}", session.id)))?;
        store.set_session_worker_id(sid, Some(StoreWorkerId(w.0))).await?;
    }
    print_json_or_text(json, &session, || session.id.to_string())
}

pub(super) async fn list_orchestrator_sessions(tenant: Option<&str>, status: &str, limit: u32, json: bool) -> ArgoResult<()> {
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

pub(super) async fn show_orchestrator_session(tenant: Option<&str>, id: &str, json: bool) -> ArgoResult<()> {
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
        repo.list_pending_handoffs_for_worker(tenant, WorkerId::from_str(w).map_err(|e| ArgoError::Schema(format!("invalid worker id '{w}': {e}")))?).await?
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
    let tenant = tenant_id(tenant)?; let repo = orchestrator_repo().await?; let hid = HandoffId::from_string(id.to_string());
    let Some(handoff) = repo.find_handoff(tenant, &hid).await? else { return Err(ArgoError::Provider(format!("no handoff {id}"))); };
    if json { println!("{}", serde_json::to_string(&handoff_json(&handoff)).unwrap_or_default()); } else { println!("{}	{:?}	{}", handoff.id, handoff.status, handoff.expected_outcome); }
    Ok(())
}

async fn cancel_handoff(tenant: Option<&str>, id: &str, note: Option<&str>, json: bool) -> ArgoResult<()> {
    let tenant = tenant_id(tenant)?; let repo = orchestrator_repo().await?; let hid = HandoffId::from_string(id.to_string()); let handoff = repo.cancel_handoff(tenant, &hid, note).await?;
    if json { println!("{}", serde_json::to_string(&handoff_json(&handoff)).unwrap_or_default()); } else { println!("{}	{}	{}", handoff.id, handoff_status_label(handoff.status), handoff.expected_outcome); }
    Ok(())
}

async fn orchestrator_repo() -> ArgoResult<OrchestratorStore> { orchestrator_store_and_repo().await.map(|(_store, repo)| repo) }

async fn orchestrator_store_and_repo() -> ArgoResult<(Arc<SessionStore>, OrchestratorStore)> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let repo = OrchestratorStore::new(Arc::clone(&store));
    repo.ensure_schema().await?;
    Ok((store, repo))
}

fn tenant_id(raw: Option<&str>) -> ArgoResult<TenantId> {
    let raw = raw.map(ToOwned::to_owned).or_else(|| std::env::var("ARGO_DEFAULT_TENANT").ok())
        .ok_or_else(|| ArgoError::Provider("missing --tenant (or ARGO_DEFAULT_TENANT)".into()))?;
    uuid::Uuid::parse_str(&raw).map(TenantId).map_err(|e| ArgoError::Schema(format!("invalid tenant id '{raw}': {e}")))
}

fn parse_handoff_status(raw: &str) -> ArgoResult<HandoffStatus> {
    match raw {
        "pending" => Ok(HandoffStatus::Pending),
        "acknowledged" => Ok(HandoffStatus::Acknowledged),
        "settled" => Ok(HandoffStatus::Settled),
        "expired" => Ok(HandoffStatus::Expired),
        "cancelled" => Ok(HandoffStatus::Cancelled),
        other => Err(ArgoError::Schema(format!("invalid handoff status '{other}'"))),
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
    match s { HandoffStatus::Pending => "pending", HandoffStatus::Acknowledged => "acknowledged", HandoffStatus::Settled => "settled", HandoffStatus::Expired => "expired", HandoffStatus::Cancelled => "cancelled" }
}

fn print_json_or_text<T: serde::Serialize>(json: bool, value: &T, text: impl FnOnce() -> String) -> ArgoResult<()> {
    if json { println!("{}", serde_json::to_string(value).unwrap_or_default()); } else { println!("{}", text()); }
    Ok(())
}

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
        let h = Handoff::new(session.id.clone(), WorkerId::new(), EscalationStep::Worker(WorkerId::new()), "finish demo", serde_json::json!({"case":"demo"})); repo.insert_handoff(tenant, &h).await.unwrap(); let rows = repo.list_handoffs_for_session(tenant, &session.id).await.unwrap(); let v = handoff_json(&rows[0]);
        assert_eq!(v["status"], "pending"); assert_eq!(v["to"]["kind"], "worker"); assert_eq!(v["payload"]["case"], "demo");
    }
}
