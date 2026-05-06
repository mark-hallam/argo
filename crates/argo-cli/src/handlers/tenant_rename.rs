//! Tenant rename support for `argo tenant`.

use super::tenant_archive::{open_store, parse_tenant, primary_workspace_id, print_json};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
struct TenantRenameOut {
    tenant_id: String,
    name: String,
    renamed_at: i64,
}

pub async fn run(id: &str, to: &str, json_out: bool) -> ArgoResult<()> {
    let tenant = parse_tenant(id)?;
    let (store, _) = open_store().await?;
    let out = rename_tenant(&store, tenant, to).await?;
    if json_out {
        print_json(&json!({"tenant_rename": out}))
    } else {
        println!("renamed tenant {} to {}", out.tenant_id, out.name);
        Ok(())
    }
}

async fn rename_tenant(
    store: &SessionStore,
    tenant: TenantId,
    name: &str,
) -> ArgoResult<TenantRenameOut> {
    let name = normalize_name(name)?;
    let workspace_id = primary_workspace_id(store, tenant).await?;
    let renamed_at = Utc::now().timestamp_micros();
    sqlx::query("UPDATE workspace SET name = ? WHERE id = ?")
        .bind(&name)
        .bind(workspace_id)
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant rename: {e}")))?;
    Ok(TenantRenameOut {
        tenant_id: tenant.to_string(),
        name,
        renamed_at,
    })
}

fn normalize_name(raw: &str) -> ArgoResult<String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(ArgoError::Provider("tenant rename: empty name".into()));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_identity::IdentityStore;
    use argo_types::WorkspaceId;
    use sqlx::Row;

    async fn store_with_tenant(name: &str) -> (SessionStore, TenantId) {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        IdentityStore::new(&store).ensure_schema().await.unwrap();
        super::super::tenant_archive::ensure_tenant_lifecycle_schema(&store)
            .await
            .unwrap();
        let tenant = TenantId::new();
        sqlx::query(
            "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
             VALUES (?, ?, ?, '.', 1)",
        )
        .bind(WorkspaceId::new().0.as_bytes().as_slice())
        .bind(tenant.0.as_bytes().as_slice())
        .bind(name)
        .execute(store.pool())
        .await
        .unwrap();
        (store, tenant)
    }

    #[test]
    fn normalize_name_rejects_blank() {
        let err = normalize_name("  ").unwrap_err();
        assert!(err.to_string().contains("empty name"));
    }

    #[tokio::test]
    async fn rename_updates_primary_workspace_name() {
        let (store, tenant) = store_with_tenant("Old").await;
        rename_tenant(&store, tenant, "New Ops").await.unwrap();

        let row = sqlx::query("SELECT name FROM workspace WHERE tenant_id = ?")
            .bind(tenant.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(row.get::<String, _>("name"), "New Ops");
    }
}
