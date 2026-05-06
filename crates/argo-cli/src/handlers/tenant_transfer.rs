//! Tenant transfer support for `argo tenant`.

use super::tenant_archive::{
    open_store, parse_tenant, primary_workspace_id, print_json, tenant_owner,
};
use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

const AUDIT_KIND: &str = "tenant_transfer";

#[derive(Debug, Serialize)]
struct TenantTransferOut {
    tenant_id: String,
    old_owner_email: Option<String>,
    new_owner_email: String,
    transferred_at: i64,
}

pub async fn run(id: &str, owner: &str, json_out: bool) -> ArgoResult<()> {
    let tenant = parse_tenant(id)?;
    let owner = normalize_owner(owner)?;
    let (store, _) = open_store().await?;
    let out = transfer_tenant(&store, tenant, &owner).await?;
    if json_out {
        print_json(&json!({"tenant_transfer": out}))
    } else {
        println!(
            "transferred tenant {} to {}",
            out.tenant_id, out.new_owner_email
        );
        Ok(())
    }
}

async fn transfer_tenant(
    store: &SessionStore,
    tenant: TenantId,
    owner: &str,
) -> ArgoResult<TenantTransferOut> {
    let workspace_id = primary_workspace_id(store, tenant).await?;
    let old_owner = tenant_owner(store, tenant).await?;
    let transferred_at = Utc::now().timestamp_micros();
    sqlx::query("UPDATE workspace SET owner_email = ? WHERE tenant_id = ?")
        .bind(owner)
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant transfer: {e}")))?;
    append_transfer_audit(
        store,
        tenant,
        workspace_id,
        old_owner.as_deref(),
        owner,
        transferred_at,
    )
    .await?;
    Ok(TenantTransferOut {
        tenant_id: tenant.to_string(),
        old_owner_email: old_owner,
        new_owner_email: owner.to_string(),
        transferred_at,
    })
}

fn normalize_owner(raw: &str) -> ArgoResult<String> {
    let owner = raw.trim();
    if owner.is_empty() || !owner.contains('@') {
        return Err(ArgoError::Provider(
            "tenant transfer: pass --owner <email>".into(),
        ));
    }
    Ok(owner.to_ascii_lowercase())
}

async fn append_transfer_audit(
    store: &SessionStore,
    tenant: TenantId,
    workspace_id: Vec<u8>,
    old_owner: Option<&str>,
    new_owner: &str,
    transferred_at: i64,
) -> ArgoResult<()> {
    let parent = last_global_hash(store).await?;
    let payload = json!({
        "tenant_id": tenant.to_string(),
        "workspace_id": Uuid::from_slice(&workspace_id)
            .map_err(|e| ArgoError::Provider(format!("workspace id blob: {e}")))?
            .to_string(),
        "old_owner_email": old_owner,
        "new_owner_email": new_owner,
        "transferred_at": transferred_at,
    });
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Provider(format!("tenant transfer audit JSON: {e}")))?;
    let self_hash = compute_chain_hash(&parent, AUDIT_KIND, &canonical);
    sqlx::query(
        "INSERT INTO audit_record
         (id, session_id, kind, payload_json, parent_hash, self_hash, created_at)
         VALUES (?, NULL, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .bind(AUDIT_KIND)
    .bind(canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .bind(transferred_at)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant transfer audit: {e}")))?;
    Ok(())
}

async fn last_global_hash(store: &SessionStore) -> ArgoResult<[u8; 32]> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant transfer last hash: {e}")))?;
    Ok(bytes.as_deref().map_or([0_u8; 32], hash32))
}

fn hash32(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0_u8; 32];
    if bytes.len() == 32 {
        out.copy_from_slice(bytes);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_identity::IdentityStore;
    use argo_types::WorkspaceId;
    use sqlx::Row;

    async fn store_with_tenant() -> (SessionStore, TenantId) {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        IdentityStore::new(&store).ensure_schema().await.unwrap();
        super::super::tenant_archive::ensure_tenant_lifecycle_schema(&store)
            .await
            .unwrap();
        let tenant = TenantId::new();
        sqlx::query(
            "INSERT INTO workspace (id, tenant_id, name, root_path, created_at, owner_email)
             VALUES (?, ?, 'Acme', '.', 1, 'old@example.test')",
        )
        .bind(WorkspaceId::new().0.as_bytes().as_slice())
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();
        (store, tenant)
    }

    #[test]
    fn normalize_owner_rejects_non_email() {
        let err = normalize_owner("operator").unwrap_err();
        assert!(err.to_string().contains("--owner"));
    }

    #[tokio::test]
    async fn transfer_updates_owner_and_writes_audit() {
        let (store, tenant) = store_with_tenant().await;
        let out = transfer_tenant(&store, tenant, "new@example.test")
            .await
            .unwrap();
        assert_eq!(out.old_owner_email.as_deref(), Some("old@example.test"));

        let owner: String =
            sqlx::query_scalar("SELECT owner_email FROM workspace WHERE tenant_id = ?")
                .bind(tenant.0.as_bytes().as_slice())
                .fetch_one(store.pool())
                .await
                .unwrap();
        assert_eq!(owner, "new@example.test");

        let row = sqlx::query(
            "SELECT kind, payload_json FROM audit_record WHERE kind = 'tenant_transfer'",
        )
        .fetch_one(store.pool())
        .await
        .unwrap();
        assert_eq!(row.get::<String, _>("kind"), "tenant_transfer");
        let payload: serde_json::Value =
            serde_json::from_str(&row.get::<String, _>("payload_json")).unwrap();
        assert_eq!(payload["old_owner_email"], "old@example.test");
        assert_eq!(payload["new_owner_email"], "new@example.test");
    }
}
