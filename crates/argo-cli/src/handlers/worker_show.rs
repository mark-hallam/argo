//! `argo worker show` rich worker introspection. Read-only report
//! over identity, channels, sessions, audit, duties, rosters, and
//! spend. Reuses the existing `WorkerCmd::Show` variant.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{IdentityStore, Roster, RosterStore, Worker, WorkerId};
use argo_orchestrator::{DutyRecord, DutyStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

const WORKER_BUDGET_SCHEMA: &str =
    include_str!("../../../argo-store/migrations/14__worker_budget.sql");

const RECENT_SESSIONS: u32 = 5;
const RECENT_AUDIT: u32 = 10;

/// Aggregated read-only snapshot rendered by `argo worker show`.
#[derive(Debug, Serialize)]
pub struct WorkerShowReport {
    pub worker: Worker,
    pub recent_sessions: Vec<SessionRow>,
    pub recent_audit: Vec<AuditRow>,
    pub duties: Vec<DutyRow>,
    pub rosters: Vec<RosterRow>,
    pub spend_ytd_usd_micros: u64,
    pub generated_at: String,
}

#[derive(Debug, Serialize)]
pub struct SessionRow {
    pub id: String,
    pub vertical_id: Option<String>,
    pub started_at: String,
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditRow {
    pub seq: i64,
    pub kind: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct DutyRow {
    pub id: String,
    pub cron: String,
    pub entry_action: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct RosterRow {
    pub id: String,
    pub name: String,
    pub member_count: usize,
    pub budget_cap_usd: Option<f64>,
}

/// Entry point invoked from `WorkerCmd::Show` dispatch.
pub async fn run(tenant: &str, worker_id: &str, json_out: bool) -> ArgoResult<()> {
    let ctx = Context::open().await?;
    let tenant_id = parse_tenant(tenant)?;
    let worker_id = parse_worker(worker_id)?;
    let report = assemble_report(&ctx, tenant_id, worker_id, Utc::now()).await?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope(&report))
                .map_err(|e| ArgoError::Provider(format!("serialize worker show: {e}")))?
        );
    } else {
        print!("{}", super::worker_show_render::render_markdown(&report));
    }
    Ok(())
}

struct Context {
    store: Arc<SessionStore>,
}

impl Context {
    async fn open() -> ArgoResult<Self> {
        let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
        let home = resolve_home(&cwd)?;
        ensure_dir(&home)?;
        let store = SessionStore::open(db_path(&home)).await?;
        store.migrate().await?;
        Ok(Self {
            store: Arc::new(store),
        })
    }
}

async fn assemble_report(
    ctx: &Context,
    tenant: TenantId,
    worker_id: WorkerId,
    now: DateTime<Utc>,
) -> ArgoResult<WorkerShowReport> {
    let identity = IdentityStore::new(ctx.store.as_ref());
    identity.ensure_schema().await?;
    let worker = identity
        .get_worker(tenant, worker_id)
        .await?
        .ok_or_else(|| {
            ArgoError::Provider(format!("no worker '{worker_id}' for tenant '{tenant}'"))
        })?;

    let store_for_sessions = Arc::clone(&ctx.store);
    let store_for_audit = Arc::clone(&ctx.store);
    let store_for_duties = Arc::clone(&ctx.store);
    let store_for_spend = Arc::clone(&ctx.store);
    let store_for_rosters = Arc::clone(&ctx.store);

    let worker_id_str = worker.id.to_string();
    let worker_id_str_for_spend = worker_id_str.clone();
    let year_start = year_start_secs(now);

    let (sessions, audit, duties, rosters, spend_ytd) = tokio::join!(
        load_recent_sessions(store_for_sessions, tenant, worker_id),
        load_recent_audit(store_for_audit, worker_id_str),
        load_recent_duties(store_for_duties, tenant, worker_id),
        load_rosters(store_for_rosters, tenant, worker_id),
        async move {
            ensure_budget_schema(store_for_spend.as_ref()).await?;
            spend_since(store_for_spend.as_ref(), &worker_id_str_for_spend, year_start).await
        },
    );

    Ok(WorkerShowReport {
        worker,
        recent_sessions: sessions.unwrap_or_default(),
        recent_audit: audit.unwrap_or_default(),
        duties: duties.unwrap_or_default(),
        rosters: rosters.unwrap_or_default(),
        spend_ytd_usd_micros: spend_ytd.unwrap_or(0),
        generated_at: now.to_rfc3339(),
    })
}

async fn load_recent_sessions(
    store: Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<Vec<SessionRow>> {
    let columns = session_columns(store.as_ref()).await?;
    let worker_blob = worker_id.0.as_bytes().to_vec();
    let worker_text = worker_id.to_string();
    let like_worker = format!("%{worker_text}%");
    let rows = if columns.iter().any(|c| c == "worker_id")
        && columns.iter().any(|c| c == "participants")
    {
        sqlx::query(
            "SELECT id, vertical_id, started_at, title FROM session \
             WHERE tenant_id = ? AND (worker_id = ? OR participants LIKE ?) \
             ORDER BY started_at DESC LIMIT ?",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .bind(worker_blob.as_slice())
        .bind(&like_worker)
        .bind(i64::from(RECENT_SESSIONS))
        .fetch_all(store.pool())
        .await
    } else if columns.iter().any(|c| c == "worker_id") {
        sqlx::query(
            "SELECT id, vertical_id, started_at, title FROM session \
             WHERE tenant_id = ? AND worker_id = ? \
             ORDER BY started_at DESC LIMIT ?",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .bind(worker_blob.as_slice())
        .bind(i64::from(RECENT_SESSIONS))
        .fetch_all(store.pool())
        .await
    } else if columns.iter().any(|c| c == "participants") {
        sqlx::query(
            "SELECT id, vertical_id, started_at, title FROM session \
             WHERE tenant_id = ? AND participants LIKE ? \
             ORDER BY started_at DESC LIMIT ?",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .bind(&like_worker)
        .bind(i64::from(RECENT_SESSIONS))
        .fetch_all(store.pool())
        .await
    } else {
        return Ok(Vec::new());
    }
    .map_err(|e| ArgoError::Provider(format!("worker show sessions: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let id: Vec<u8> = row.get("id");
            let started_at: i64 = row.get("started_at");
            SessionRow {
                id: uuid_from_blob(&id).to_string(),
                vertical_id: row.get("vertical_id"),
                started_at: format_micros(started_at),
                title: row.get("title"),
            }
        })
        .collect())
}

async fn session_columns(store: &SessionStore) -> ArgoResult<Vec<String>> {
    sqlx::query_scalar("SELECT name FROM pragma_table_info('session')")
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("worker show session schema: {e}")))
}

async fn load_recent_audit(
    store: Arc<SessionStore>,
    worker_id: String,
) -> ArgoResult<Vec<AuditRow>> {
    let rows = sqlx::query(
        "SELECT seq, kind, created_at FROM audit_record \
         WHERE json_extract(payload_json, '$.worker_id') = ? \
            OR json_extract(payload_json, '$.from_worker_id') = ? \
            OR json_extract(payload_json, '$.to.worker_id') = ? \
         ORDER BY seq DESC LIMIT ?",
    )
    .bind(&worker_id)
    .bind(&worker_id)
    .bind(&worker_id)
    .bind(i64::from(RECENT_AUDIT))
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("worker show audit: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let micros: i64 = row.get("created_at");
            AuditRow {
                seq: row.get("seq"),
                kind: row.get("kind"),
                created_at: format_micros(micros),
            }
        })
        .collect())
}

async fn load_recent_duties(
    store: Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<Vec<DutyRow>> {
    let duty_store = DutyStore::new(store);
    let all = duty_store.list_for_tenant(tenant).await?;
    Ok(all
        .into_iter()
        .filter(|d: &DutyRecord| d.worker_id == worker_id)
        .map(|d| DutyRow {
            id: d.id,
            cron: d.cron,
            entry_action: d.entry_action,
            description: d.description,
            enabled: d.paused_until.is_none(),
        })
        .collect())
}

async fn load_rosters(
    store: Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<Vec<RosterRow>> {
    let roster_store = RosterStore::new(store);
    roster_store.ensure_schema().await?;
    let rosters = roster_store.rosters_for_worker(tenant, worker_id).await?;
    Ok(rosters.into_iter().map(roster_row).collect())
}

fn roster_row(r: Roster) -> RosterRow {
    RosterRow {
        id: r.id.to_string(),
        name: r.name,
        member_count: r.member_worker_ids.len(),
        budget_cap_usd: r.budget_cap_usd,
    }
}

async fn ensure_budget_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in WORKER_BUDGET_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if !sql.is_empty() {
            sqlx::query(sql)
                .execute(store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("worker show budget schema: {e}")))?;
        }
    }
    Ok(())
}

async fn spend_since(store: &SessionStore, worker_id: &str, since: i64) -> ArgoResult<u64> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \
         WHERE worker_id = ? AND charged_at >= ?",
    )
    .bind(worker_id)
    .bind(since)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("worker show spend: {e}")))?;
    Ok(u64::try_from(total).unwrap_or(0))
}

fn year_start_secs(now: DateTime<Utc>) -> i64 {
    Utc.with_ymd_and_hms(now.year(), 1, 1, 0, 0, 0)
        .single()
        .unwrap_or(now)
        .timestamp()
}

fn envelope(r: &WorkerShowReport) -> Value {
    json!({"ok": true, "worker_show": r, "rosters": r.rosters})
}

fn format_micros(micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(micros)
        .unwrap_or_else(Utc::now)
        .to_rfc3339()
}

fn uuid_from_blob(blob: &[u8]) -> Uuid {
    let mut bytes = [0_u8; 16];
    let n = blob.len().min(16);
    bytes[..n].copy_from_slice(&blob[..n]);
    Uuid::from_bytes(bytes)
}

fn parse_tenant(value: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(value)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{value}': {e}")))
}

fn parse_worker(value: &str) -> ArgoResult<WorkerId> {
    value
        .parse()
        .map_err(|e| ArgoError::Provider(format!("worker id '{value}': {e}")))
}

#[cfg(test)]
pub(crate) fn report_for_test(
    worker: Worker,
    sessions: Vec<SessionRow>,
    audit: Vec<AuditRow>,
    duties: Vec<DutyRow>,
    spend_ytd_usd_micros: u64,
    generated_at: DateTime<Utc>,
) -> WorkerShowReport {
    WorkerShowReport {
        worker,
        recent_sessions: sessions,
        recent_audit: audit,
        duties,
        rosters: Vec::new(),
        spend_ytd_usd_micros,
        generated_at: generated_at.to_rfc3339(),
    }
}

#[cfg(test)]
pub(crate) fn envelope_for_test(r: &WorkerShowReport) -> Value {
    envelope(r)
}
