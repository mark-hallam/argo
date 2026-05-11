//! `argo tenant` management commands.

use super::tenant_summary::{
    self, sessions, summary, tenant_lifecycle, workers, SessionSummary, TenantSummary,
    WorkerSummary,
};
use crate::cli::TenantCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::IdentityStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use chrono::Utc;
use serde::Serialize;
use sqlx::Row;
use std::collections::BTreeMap;
use std::path::PathBuf;
use uuid::Uuid;

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
        TenantCmd::RateLimit { cmd } => super::tenant_rate_limit::run(cmd).await,
    }
}

#[allow(dead_code)]
async fn list(include_archived: bool, json: bool) -> ArgoResult<()> {
    let (store, _) = open_store().await?;
    let rows = tenant_ids(&store, include_archived).await?;
    let mut out = Vec::with_capacity(rows.len());
    for tenant in rows {
        out.push(tenant_summary::summary(&store, tenant).await?);
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
    Uuid::from_slice(raw)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant uuid blob: {e}")))
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max.saturating_sub(3)).collect();
    out.push_str("...");
    out
}
