//! `argo tenant` management commands.

use crate::cli::TenantCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::IdentityStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use std::collections::BTreeMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct TenantSummary {
    id: String,
    name: Option<String>,
    owner_email: Option<String>,
    archived_at: Option<i64>,
    archived_at_rfc3339: Option<String>,
    workspace_count: i64,
    worker_count: i64,
    session_count: i64,
    last_action_at: Option<i64>,
    last_action_at_rfc3339: Option<String>,
}

#[derive(Debug, Serialize)]
struct TenantInfo {
    tenant: TenantSummary,
    workers: Vec<WorkerSummary>,
    sessions: Vec<SessionSummary>,
}

#[derive(Debug, Serialize)]
struct TenantCreated {
    id: String,
    name: String,
    workspace_id: String,
    root_path: String,
    created_at: i64,
}

#[derive(Debug, Serialize)]
struct WorkerSummary {
    id: String,
    vertical_id: String,
    persona: String,
    created_at: i64,
    created_at_rfc3339: Option<String>,
}

#[derive(Debug, Serialize)]
struct SessionSummary {
    id: String,
    title: Option<String>,
    model: String,
    provider: String,
    started_at: i64,
    started_at_rfc3339: Option<String>,
}

pub async fn run(cmd: TenantCmd) -> ArgoResult<()> {
    match cmd {
        TenantCmd::List {
            include_archived,
            json,
        } => super::tenant_list::run(include_archived, json).await,
        TenantCmd::Info { id, json } => info(&id, json).await,
        TenantCmd::Show { id, json } => super::tenant_show::run(&id, json).await,
        TenantCmd::Create { name, json } => create(&name, json).await,
        TenantCmd::Rename {
            id,
            new_id,
            to,
            json,
        } => match (new_id, to) {
            (Some(new_id), None) => super::tenant_rename::run_id(&id, &new_id, json).await,
            (None, Some(to)) => super::tenant_rename::run(&id, &to, json).await,
            (Some(_), Some(_)) => Err(ArgoError::Provider(
                "tenant rename: pass either NEW_ID or --to, not both".into(),
            )),
            (None, None) => Err(ArgoError::Provider(
                "tenant rename: missing NEW_ID (or use --to NEW_NAME)".into(),
            )),
        },
        TenantCmd::Archive { id, json } => super::tenant_archive::run(&id, json).await,
        TenantCmd::Transfer { id, owner, json } => {
            super::tenant_transfer::run(&id, &owner, json).await
        }
        TenantCmd::Quota { cmd } => super::tenant_quota_set::run_cmd(cmd).await,
    }
}

#[allow(dead_code)]
async fn list(include_archived: bool, json: bool) -> ArgoResult<()> {
    let (store, _) = open_store().await?;
    let rows = tenant_ids(&store, include_archived).await?;
    let mut out = Vec::with_capacity(rows.len());
    for tenant in rows {
        out.push(summary(&store, tenant).await?);
    }
    if json {
        print_json(&out)
    } else {
        print_tenant_table(&out);
        Ok(())
    }
}

async fn info(raw: &str, json: bool) -> ArgoResult<()> {
    let tenant = parse_tenant(raw)?;
    let (store, _) = open_store().await?;
    let tenant = summary(&store, tenant).await?;
    if tenant.workspace_count == 0 && tenant.worker_count == 0 && tenant.session_count == 0 {
        return Err(ArgoError::Provider(format!("no tenant '{raw}'")));
    }
    let parsed = parse_tenant(&tenant.id)?;
    let out = TenantInfo {
        tenant,
        workers: workers(&store, parsed).await?,
        sessions: sessions(&store, parsed).await?,
    };
    if json {
        print_json(&out)
    } else {
        print_info(&out);
        Ok(())
    }
}

async fn create(name: &str, json: bool) -> ArgoResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ArgoError::Provider("tenant create: empty name".into()));
    }
    let (store, cwd) = open_store().await?;
    let tenant = TenantId::new();
    let workspace = WorkspaceId::new();
    let root_path = cwd.to_string_lossy().to_string();
    let created_at = Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(name)
    .bind(&root_path)
    .bind(created_at)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant create: {e}")))?;
    let out = TenantCreated {
        id: tenant.to_string(),
        name: name.to_string(),
        workspace_id: workspace.to_string(),
        root_path,
        created_at,
    };
    if json {
        print_json(&out)
    } else {
        println!("created tenant {} ({})", out.id, out.name);
        Ok(())
    }
}

async fn open_store() -> ArgoResult<(SessionStore, PathBuf)> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    IdentityStore::new(&store).ensure_schema().await?;
    super::tenant_archive::ensure_tenant_lifecycle_schema(&store).await?;
    Ok((store, cwd))
}

async fn tenant_ids(store: &SessionStore, include_archived: bool) -> ArgoResult<Vec<TenantId>> {
    let mut out = BTreeMap::new();
    collect_blob_tenants(store, "SELECT DISTINCT tenant_id FROM workspace", &mut out).await?;
    collect_blob_tenants(store, "SELECT DISTINCT tenant_id FROM session", &mut out).await?;
    let rows = sqlx::query("SELECT DISTINCT tenant_id FROM worker")
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant list workers: {e}")))?;
    for row in rows {
        let raw: String = row.get("tenant_id");
        let tenant = parse_tenant(&raw)?;
        out.insert(tenant.to_string(), tenant);
    }
    let tenants = out.into_values();
    if include_archived {
        return Ok(tenants.collect());
    }
    let mut visible = Vec::new();
    for tenant in tenants {
        if tenant_lifecycle(store, tenant).await?.archived_at.is_none() {
            visible.push(tenant);
        }
    }
    Ok(visible)
}

async fn collect_blob_tenants(
    store: &SessionStore,
    query: &str,
    out: &mut BTreeMap<String, TenantId>,
) -> ArgoResult<()> {
    let rows = sqlx::query(query)
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant list: {e}")))?;
    for row in rows {
        let raw: Vec<u8> = row.get("tenant_id");
        let tenant = tenant_from_blob(&raw)?;
        out.insert(tenant.to_string(), tenant);
    }
    Ok(())
}

async fn summary(store: &SessionStore, tenant: TenantId) -> ArgoResult<TenantSummary> {
    let workspace_count = count_blob(store, "workspace", tenant).await?;
    let session_count = count_blob(store, "session", tenant).await?;
    let worker_count = count_text(store, "worker", tenant).await?;
    let last_action_at = last_action_at(store, tenant).await?;
    let lifecycle = tenant_lifecycle(store, tenant).await?;
    Ok(TenantSummary {
        id: tenant.to_string(),
        name: tenant_name(store, tenant).await?,
        owner_email: lifecycle.owner_email,
        archived_at: lifecycle.archived_at,
        archived_at_rfc3339: lifecycle.archived_at.and_then(format_micros),
        workspace_count,
        worker_count,
        session_count,
        last_action_at,
        last_action_at_rfc3339: last_action_at.and_then(format_micros),
    })
}

#[derive(Debug)]
struct TenantLifecycle {
    owner_email: Option<String>,
    archived_at: Option<i64>,
}

async fn tenant_lifecycle(store: &SessionStore, tenant: TenantId) -> ArgoResult<TenantLifecycle> {
    let row = sqlx::query(
        "SELECT owner_email, archived_at FROM workspace \
         WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant lifecycle: {e}")))?;
    Ok(row.map_or(
        TenantLifecycle {
            owner_email: None,
            archived_at: None,
        },
        |row| TenantLifecycle {
            owner_email: row.get("owner_email"),
            archived_at: row.get("archived_at"),
        },
    ))
}

async fn tenant_name(store: &SessionStore, tenant: TenantId) -> ArgoResult<Option<String>> {
    sqlx::query_scalar(
        "SELECT name FROM workspace WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant name: {e}")))
}

async fn count_blob(store: &SessionStore, table: &str, tenant: TenantId) -> ArgoResult<i64> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant count {table}: {e}")))
}

async fn count_text(store: &SessionStore, table: &str, tenant: TenantId) -> ArgoResult<i64> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.to_string())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant count {table}: {e}")))
}

async fn last_action_at(store: &SessionStore, tenant: TenantId) -> ArgoResult<Option<i64>> {
    let mut values = Vec::new();
    values.push(
        max_blob(
            store,
            "SELECT MAX(started_at) FROM session WHERE tenant_id = ?",
            tenant,
        )
        .await?,
    );
    values.push(
        max_blob(
            store,
            "SELECT MAX(ended_at) FROM session WHERE tenant_id = ?",
            tenant,
        )
        .await?,
    );
    values.push(max_blob(store, "SELECT MAX(m.created_at) FROM message m JOIN session s ON s.id = m.session_id WHERE s.tenant_id = ?", tenant).await?);
    values.push(max_blob(store, "SELECT MAX(a.created_at) FROM audit_record a JOIN session s ON s.id = a.session_id WHERE s.tenant_id = ?", tenant).await?);
    values.push(
        max_text(
            store,
            "SELECT MAX(created_at) FROM worker WHERE tenant_id = ?",
            tenant,
        )
        .await?,
    );
    Ok(values.into_iter().flatten().max())
}

async fn max_blob(store: &SessionStore, sql: &str, tenant: TenantId) -> ArgoResult<Option<i64>> {
    sqlx::query_scalar(sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant max: {e}")))
}

async fn max_text(store: &SessionStore, sql: &str, tenant: TenantId) -> ArgoResult<Option<i64>> {
    sqlx::query_scalar(sql)
        .bind(tenant.to_string())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant max: {e}")))
}

async fn workers(store: &SessionStore, tenant: TenantId) -> ArgoResult<Vec<WorkerSummary>> {
    let rows = sqlx::query(
        "SELECT w.id, w.vertical_id, p.name AS persona, w.created_at \
         FROM worker w JOIN persona p ON p.id = w.persona_id \
         WHERE w.tenant_id = ? ORDER BY w.created_at DESC, w.id ASC LIMIT 25",
    )
    .bind(tenant.to_string())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant workers: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let created_at = row.get("created_at");
            WorkerSummary {
                id: row.get("id"),
                vertical_id: row.get("vertical_id"),
                persona: row.get("persona"),
                created_at,
                created_at_rfc3339: format_micros(created_at),
            }
        })
        .collect())
}

async fn sessions(store: &SessionStore, tenant: TenantId) -> ArgoResult<Vec<SessionSummary>> {
    let rows = sqlx::query(
        "SELECT id, title, model, provider, started_at FROM session \
         WHERE tenant_id = ? ORDER BY started_at DESC, id ASC LIMIT 25",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant sessions: {e}")))?;
    rows.into_iter()
        .map(|row| {
            let id: Vec<u8> = row.get("id");
            let started_at = row.get("started_at");
            Ok(SessionSummary {
                id: uuid_from_blob(&id)?.to_string(),
                title: row.get("title"),
                model: row.get("model"),
                provider: row.get("provider"),
                started_at,
                started_at_rfc3339: format_micros(started_at),
            })
        })
        .collect()
}

fn print_tenant_table(rows: &[TenantSummary]) {
    if rows.is_empty() {
        println!("(no tenants)");
        return;
    }
    println!(
        "{:<36}  {:<20}  {:<24}  {:<20}  workspaces  workers  sessions  last_action",
        "tenant_id", "name", "owner", "archived"
    );
    for row in rows {
        println!(
            "{:<36}  {:<20}  {:<24}  {:<20}  {:>10}  {:>7}  {:>8}  {}",
            row.id,
            truncate(row.name.as_deref().unwrap_or(""), 20),
            truncate(row.owner_email.as_deref().unwrap_or(""), 24),
            row.archived_at_rfc3339.as_deref().unwrap_or("-"),
            row.workspace_count,
            row.worker_count,
            row.session_count,
            row.last_action_at_rfc3339.as_deref().unwrap_or("-")
        );
    }
}

fn print_info(out: &TenantInfo) {
    println!("tenant {}", out.tenant.id);
    println!(
        "  name:          {}",
        out.tenant.name.as_deref().unwrap_or("-")
    );
    println!("  workers:       {}", out.tenant.worker_count);
    println!("  sessions:      {}", out.tenant.session_count);
    println!(
        "  last_action:   {}",
        out.tenant.last_action_at_rfc3339.as_deref().unwrap_or("-")
    );
    for worker in &out.workers {
        println!(
            "  worker {} {} {}",
            worker.id, worker.persona, worker.vertical_id
        );
    }
    for session in &out.sessions {
        println!(
            "  session {} {}",
            session.id,
            session.title.as_deref().unwrap_or("(untitled)")
        );
    }
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize tenant JSON: {e}")))?;
    println!("{body}");
    Ok(())
}

fn parse_tenant(raw: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(raw)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{raw}': {e}")))
}

fn tenant_from_blob(raw: &[u8]) -> ArgoResult<TenantId> {
    uuid_from_blob(raw).map(TenantId)
}

fn uuid_from_blob(raw: &[u8]) -> ArgoResult<Uuid> {
    Uuid::from_slice(raw).map_err(|e| ArgoError::Provider(format!("tenant uuid blob: {e}")))
}

fn format_micros(micros: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_micros(micros).map(|ts| ts.to_rfc3339())
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max.saturating_sub(3)).collect();
    out.push_str("...");
    out
}
