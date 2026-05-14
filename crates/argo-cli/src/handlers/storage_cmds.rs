//! `argo storage` tenant-residency operator commands.
use crate::cli::StorageCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_config::{ArgoConfig, StorageTenantConfig};
use argo_store::multi_tenant_pool::{MultiTenantStore, TenantPoolConfig};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(cmd: StorageCmd) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let cfg = ArgoConfig::load(&cwd.join("argo.toml"))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let default = SessionStore::open(db_path(&home)).await?;
    let store = MultiTenantStore::open(default, tenant_configs(&cfg)).await?;
    match cmd {
        StorageCmd::ListTenants { json } | StorageCmd::Health { json } => {
            let rows = store.tenant_statuses().await;
            if json {
                println!("{}", serde_json::to_string(&rows).unwrap_or_default());
            } else if rows.is_empty() {
                println!("no tenant-specific storage pools configured");
            } else {
                println!("tenant_id\tregion\tschema_version\thealth\tdb_url");
                for row in rows {
                    let version = row
                        .schema_version
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "-".into());
                    let health = if row.healthy { "ok" } else { "error" };
                    println!(
                        "{}\t{}\t{}\t{}\t{}",
                        row.tenant_id, row.region, version, health, row.db_url
                    );
                }
            }
        }
        StorageCmd::MigrateTenant { tenant_id, json } => {
            if !store.has_tenant(&tenant_id) {
                return Err(ArgoError::Provider(format!(
                    "tenant '{tenant_id}' has no [storage.tenants.{tenant_id}] entry"
                )));
            }
            let version = store.migrate_tenant(&tenant_id).await?;
            if json {
                let payload = serde_json::json!({
                    "tenant_id": tenant_id,
                    "schema_version": version,
                    "migrated": true,
                });
                println!("{}", serde_json::to_string(&payload).unwrap_or_default());
            } else {
                println!("tenant {tenant_id} migrated to schema v{version}");
            }
        }
    }
    Ok(())
}

fn tenant_configs(cfg: &ArgoConfig) -> Vec<TenantPoolConfig> {
    cfg.storage
        .tenants
        .iter()
        .map(|(tenant_id, tenant)| tenant_config(tenant_id, tenant))
        .collect()
}

fn tenant_config(tenant_id: &str, tenant: &StorageTenantConfig) -> TenantPoolConfig {
    TenantPoolConfig {
        tenant_id: tenant_id.to_string(),
        db_url: tenant.db_url.clone(),
        region: tenant.region.clone(),
    }
}
