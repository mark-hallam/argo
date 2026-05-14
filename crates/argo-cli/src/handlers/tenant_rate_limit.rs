//! `argo tenant rate-limit set/get/list/delete` (Stream
//! `mission-per-tenant-rate-limiting`, MISSION_BACKLOG Section 1
//! #21). Per-tenant overrides live in the workspace
//! `tenant_rate_limit` table; the gateway middleware consults the
//! map at acquire time, falling back to the
//! `[gateway.rate_limit.tenant]` config default when the row is
//! absent.

use crate::cli::cli_tenant_cmds::TenantRateLimitCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::tenant_rate_limit::{TenantRateLimitOverride, TenantRateLimitStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

pub async fn run(cmd: TenantRateLimitCmd) -> ArgoResult<()> {
    match cmd {
        TenantRateLimitCmd::Set { tenant, rpm, json } => set(&tenant, rpm, json).await,
        TenantRateLimitCmd::Get { tenant, json } => get(&tenant, json).await,
        TenantRateLimitCmd::List { json } => list(json).await,
        TenantRateLimitCmd::Delete { tenant, json } => delete(&tenant, json).await,
    }
}

async fn set(tenant_raw: &str, rpm: u32, json_out: bool) -> ArgoResult<()> {
    if rpm == 0 {
        return Err(ArgoError::Provider(
            "tenant rate-limit set: --rpm must be > 0 (use `delete` to remove an override)".into(),
        ));
    }
    let tenant = parse_tenant(tenant_raw)?;
    let store = open_store().await?;
    let now = Utc::now().timestamp();
    TenantRateLimitStore::new(store.pool())
        .set(tenant, rpm, now)
        .await?;
    let row = TenantRateLimitOverride {
        tenant_id: tenant,
        requests_per_minute: rpm,
        updated_at: now,
    };
    if json_out {
        print_json(&json!({"ok": true, "override": to_dto(&row)}))
    } else {
        println!(
            "set tenant_rate_limit tenant={} rpm={} updated_at={}",
            row.tenant_id, row.requests_per_minute, row.updated_at
        );
        Ok(())
    }
}

async fn get(tenant_raw: &str, json_out: bool) -> ArgoResult<()> {
    let tenant = parse_tenant(tenant_raw)?;
    let store = open_store().await?;
    let row = TenantRateLimitStore::new(store.pool()).get(tenant).await?;
    if json_out {
        let body = row
            .as_ref()
            .map(to_dto)
            .map_or_else(|| json!(null), |dto| json!(dto));
        print_json(&json!({"tenant": tenant.to_string(), "override": body}))
    } else {
        match row {
            Some(r) => println!(
                "tenant_rate_limit tenant={} rpm={} updated_at={}",
                r.tenant_id, r.requests_per_minute, r.updated_at
            ),
            None => println!("(no override for tenant {tenant})"),
        }
        Ok(())
    }
}

async fn list(json_out: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    let mut rows = TenantRateLimitStore::new(store.pool()).list().await?;
    rows.sort_by_key(|r| r.tenant_id.to_string());
    if json_out {
        let body: Vec<_> = rows.iter().map(to_dto).collect();
        print_json(&json!({"overrides": body}))
    } else if rows.is_empty() {
        println!("(no per-tenant rate-limit overrides)");
        Ok(())
    } else {
        println!("{:<36}  {:>6}  updated_at", "tenant_id", "rpm");
        for r in &rows {
            println!(
                "{:<36}  {:>6}  {}",
                r.tenant_id, r.requests_per_minute, r.updated_at
            );
        }
        Ok(())
    }
}

async fn delete(tenant_raw: &str, json_out: bool) -> ArgoResult<()> {
    let tenant = parse_tenant(tenant_raw)?;
    let store = open_store().await?;
    let removed = TenantRateLimitStore::new(store.pool())
        .delete(tenant)
        .await?;
    if json_out {
        print_json(&json!({"ok": true, "removed": removed, "tenant": tenant.to_string()}))
    } else if removed {
        println!("deleted tenant_rate_limit tenant={tenant}");
        Ok(())
    } else {
        println!("(no override for tenant {tenant})");
        Ok(())
    }
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

#[derive(Serialize)]
struct OverrideDto {
    tenant_id: String,
    requests_per_minute: u32,
    updated_at: i64,
}

fn to_dto(r: &TenantRateLimitOverride) -> OverrideDto {
    OverrideDto {
        tenant_id: r.tenant_id.to_string(),
        requests_per_minute: r.requests_per_minute,
        updated_at: r.updated_at,
    }
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize JSON: {e}")))?;
    println!("{body}");
    Ok(())
}

fn parse_tenant(raw: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(raw)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{raw}': {e}")))
}
