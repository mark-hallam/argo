//! Tenant-scoped worker list/show commands.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{AutoApprovalPolicy, IdentityStore, Roster, RosterStore, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct ListWorkerOut {
    id: String,
    tenant_id: String,
    vertical_id: String,
    persona: String,
    role: ListRoleOut,
    auto_approval: ListPolicyOut,
    created_at: i64,
    created_at_rfc3339: String,
}

#[derive(Debug, Serialize)]
struct ListRoleOut {
    charter: String,
}

#[derive(Debug, Serialize)]
struct ListPolicyOut {
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct ShowWorkerOut {
    worker: Worker,
    created_at: i64,
    created_at_rfc3339: String,
    resolved_policy: AutoApprovalPolicy,
    rosters: Vec<RosterLineage>,
}

#[derive(Debug, Serialize)]
struct RosterLineage {
    id: String,
    name: String,
    policy_enabled: bool,
}

pub async fn list(tenant: &str, persona: Option<&str>, json: bool) -> ArgoResult<()> {
    let (store, _cwd) = open_store().await?;
    let tenant_id = parse_tenant_id(tenant)?;
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await?;
    let created = created_at_by_worker(&store, tenant_id).await?;
    let mut rows = Vec::new();

    for worker in identities.list_workers(tenant_id).await? {
        if let Some(name) = persona {
            if worker.persona.name != name {
                continue;
            }
        }
        let created_at = created
            .get(&worker.id.to_string())
            .copied()
            .unwrap_or_default();
        rows.push(ListWorkerOut::from_worker(&worker, created_at));
    }

    if json {
        print_json(&rows)
    } else {
        print_list(&rows);
        Ok(())
    }
}

#[allow(dead_code)]
pub async fn show(tenant: &str, worker_id: &str, _json: bool) -> ArgoResult<()> {
    let out = resolved_worker(tenant, worker_id).await?;
    print_json(&out)
}

pub async fn policy_show(tenant: &str, worker_id: &str, _json: bool) -> ArgoResult<()> {
    let out = resolved_worker(tenant, worker_id).await?;
    print_json(&out.resolved_policy)
}

impl ListWorkerOut {
    fn from_worker(worker: &Worker, created_at: i64) -> Self {
        Self {
            id: worker.id.to_string(),
            tenant_id: worker.tenant_id.to_string(),
            vertical_id: worker.vertical_id.clone(),
            persona: worker.persona.name.clone(),
            role: ListRoleOut {
                charter: worker.role.charter.clone(),
            },
            auto_approval: ListPolicyOut {
                enabled: worker.auto_approval.enabled,
            },
            created_at,
            created_at_rfc3339: format_micros(created_at),
        }
    }
}

async fn resolved_worker(tenant: &str, worker_id: &str) -> ArgoResult<ShowWorkerOut> {
    let (store, cwd) = open_store().await?;
    let tenant_id = parse_tenant_id(tenant)?;
    let worker_id = parse_worker_id(worker_id)?;
    let identities = IdentityStore::new(&store);
    let worker = identities
        .get_worker(tenant_id, worker_id)
        .await?
        .ok_or_else(|| {
            ArgoError::Provider(format!("no worker '{worker_id}' for tenant '{tenant}'"))
        })?;
    let workspace_default = super::worker_actions_auto_approval::load_workspace_policy(&cwd)?;
    let rosters = roster_lineage_for_worker(&store, tenant_id, worker_id).await?;
    let roster_policies: Vec<AutoApprovalPolicy> =
        rosters.iter().map(|r| r.policy.clone()).collect();
    let resolved_policy =
        worker.resolved_policy_with_rosters(&roster_policies, &workspace_default);
    let lineage = rosters
        .iter()
        .map(|r| RosterLineage {
            id: r.id.to_string(),
            name: r.name.clone(),
            policy_enabled: r.policy.enabled,
        })
        .collect();
    let created_at = created_at_for_worker(&store, tenant_id, worker_id).await?;
    Ok(ShowWorkerOut {
        worker,
        created_at,
        created_at_rfc3339: format_micros(created_at),
        resolved_policy,
        rosters: lineage,
    })
}

async fn roster_lineage_for_worker(
    _store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<Vec<Roster>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let pool_store = Arc::new(SessionStore::open(db_path(&home)).await?);
    pool_store.migrate().await?;
    let rosters = RosterStore::new(pool_store);
    rosters.ensure_schema().await?;
    rosters.rosters_for_worker(tenant, worker_id).await
}

async fn open_store() -> ArgoResult<(SessionStore, PathBuf)> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok((store, cwd))
}

async fn created_at_by_worker(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<HashMap<String, i64>> {
    let rows = sqlx::query("SELECT id, created_at FROM worker WHERE tenant_id = ?")
        .bind(tenant_id.to_string())
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list worker timestamps: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get("id"), row.get("created_at")))
        .collect())
}

async fn created_at_for_worker(
    store: &SessionStore,
    tenant_id: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<i64> {
    let created =
        sqlx::query_scalar("SELECT created_at FROM worker WHERE id = ? AND tenant_id = ?")
            .bind(worker_id.to_string())
            .bind(tenant_id.to_string())
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("load worker timestamp: {e}")))?;
    created.ok_or_else(|| {
        ArgoError::Provider(format!("no worker '{worker_id}' for tenant '{tenant_id}'"))
    })
}

fn print_list(rows: &[ListWorkerOut]) {
    if rows.is_empty() {
        println!("(no workers)");
        return;
    }
    println!(
        "{:<36}  {:<16}  {:<44}  {:<13}  created_at",
        "worker_id", "persona", "role.charter", "auto_approval"
    );
    for row in rows {
        println!(
            "{:<36}  {:<16}  {:<44}  {:<13}  {}",
            row.id,
            truncate(&row.persona, 16),
            truncate(&row.role.charter, 44),
            row.auto_approval.enabled,
            row.created_at_rfc3339
        );
    }
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize worker JSON: {e}")))?;
    println!("{body}");
    Ok(())
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

fn format_micros(micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(micros)
        .unwrap_or_else(Utc::now)
        .to_rfc3339()
}

fn parse_tenant_id(value: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(value)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{value}': {e}")))
}

fn parse_worker_id(value: &str) -> ArgoResult<WorkerId> {
    value
        .parse()
        .map_err(|e| ArgoError::Provider(format!("worker id '{value}': {e}")))
}
