//! `argo duty` dispatch entry point + `list` subcommand
//! (Stream `claude-worker-duty-scheduler`, V1.3). Other subcommands
//! live in sibling `duty_*.rs` files.
use crate::cli::cli_duty_cmds::DutyCmd;
use crate::handlers::{duty_fire_now, duty_pause, duty_resume, duty_show};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_orchestrator::{DutyRecord, DutyStore, OpenGate, OrchestratorStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

/// Top-level dispatch for `argo duty ...`.
pub async fn run(cmd: DutyCmd) -> ArgoResult<()> {
    match cmd {
        DutyCmd::List { tenant, json } => list(&tenant, json).await,
        DutyCmd::Show { id, json } => duty_show::run(&id, json).await,
        DutyCmd::Pause { id, until, json } => duty_pause::run(&id, until.as_deref(), json).await,
        DutyCmd::Resume { id, json } => duty_resume::run(&id, json).await,
        DutyCmd::FireNow { id, json } => duty_fire_now::run(&id, json).await,
    }
}

#[derive(Debug, Serialize)]
struct ListRow {
    id: String,
    worker_id: String,
    cron: String,
    entry_action: String,
    enabled: bool,
    paused_until: Option<String>,
    last_fired_at: Option<String>,
}

impl From<&DutyRecord> for ListRow {
    fn from(d: &DutyRecord) -> Self {
        Self {
            id: d.id.clone(),
            worker_id: d.worker_id.to_string(),
            cron: d.cron.clone(),
            entry_action: d.entry_action.clone(),
            enabled: d.enabled,
            paused_until: d.paused_until.map(|d| d.to_rfc3339()),
            last_fired_at: d.last_fired_at.map(|d| d.to_rfc3339()),
        }
    }
}

async fn list(tenant: &str, json: bool) -> ArgoResult<()> {
    let store = open_duty_store().await?;
    let tenant_id = parse_tenant(tenant)?;
    let duties = store.list_for_tenant(tenant_id).await?;
    let rows: Vec<ListRow> = duties.iter().map(ListRow::from).collect();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&rows)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        print_table(&rows);
    }
    Ok(())
}

fn print_table(rows: &[ListRow]) {
    if rows.is_empty() {
        println!("(no duties for this tenant)");
        return;
    }
    println!(
        "{:<36}  {:<24}  {:<8}  {:<8}  entry_action",
        "id", "cron", "enabled", "paused",
    );
    for row in rows {
        println!(
            "{:<36}  {:<24}  {:<8}  {:<8}  {}",
            row.id,
            truncate(&row.cron, 24),
            row.enabled,
            row.paused_until.is_some(),
            row.entry_action,
        );
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    if max <= 3 {
        return value.chars().take(max).collect();
    }
    let mut out: String = value.chars().take(max - 3).collect();
    out.push_str("...");
    out
}

/// Open a [`DutyStore`] backed by the cwd workspace.
pub(crate) async fn open_duty_store() -> ArgoResult<DutyStore> {
    let store = open_session_store().await?;
    let duties = DutyStore::new(store);
    duties.ensure_schema().await?;
    Ok(duties)
}

/// Open the underlying [`SessionStore`].
pub(crate) async fn open_session_store() -> ArgoResult<Arc<SessionStore>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(Arc::new(store))
}

/// Build a fresh [`OrchestratorStore`] sharing the cwd's
/// [`SessionStore`].
pub(crate) async fn open_orchestrator() -> ArgoResult<Arc<OrchestratorStore>> {
    let store = open_session_store().await?;
    let orch = Arc::new(OrchestratorStore::new(store));
    orch.ensure_schema().await?;
    Ok(orch)
}

/// Build an [`OpenGate`] (the gateway-default for CLI flows that
/// don't have access to the per-worker resolved policy).
pub(crate) fn default_gate() -> Arc<OpenGate> {
    Arc::new(OpenGate)
}

/// Parse a tenant uuid string.
pub(crate) fn parse_tenant(value: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(value)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{value}': {e}")))
}
