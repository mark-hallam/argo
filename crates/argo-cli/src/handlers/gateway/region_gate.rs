//! Data-residency gate for tenant-specific storage pools.
use argo_audit::{AuditKind, AuditSink};
use argo_config::ArgoConfig;
use argo_store::SessionStore;
use serde_json::json;
use std::sync::Arc;

/// Reject requests whose tenant is pinned to another deployment region.
pub(super) async fn reject_cross_region(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let deployment_region = std::env::var("ARGO_DEPLOYMENT_REGION").ok()?;
    let cfg = argo_config::load().ok()?;
    reject_cross_region_with_config(req, store, &cfg, &deployment_region).await
}

async fn reject_cross_region_with_config(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    cfg: &ArgoConfig,
    deployment_region: &str,
) -> Option<(&'static str, String)> {
    let tenant_id = super::rate_limit::extract_tenant(req);
    let tenant = cfg.storage.tenants.get(&tenant_id)?;
    if tenant.region == deployment_region {
        return None;
    }
    if let Some(store) = store {
        let payload = json!({
            "tenant_id": tenant_id,
            "required_region": tenant.region,
            "deployment_region": deployment_region,
            "route": request_line(req),
        });
        let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.clone());
        if let Err(e) = sink
            .write(None, AuditKind::CrossRegionBlocked, payload, None)
            .await
        {
            tracing::warn!(target: "gateway::region_gate", error = %e, "cross-region audit write failed");
        }
        let _ = sink.flush_pending().await;
    }
    let body = json!({
        "error": "cross_region_blocked",
        "tenant_id": tenant_id,
        "required_region": tenant.region,
        "deployment_region": deployment_region,
    })
    .to_string();
    Some(("451 Unavailable For Legal Reasons", body))
}

fn request_line(req: &str) -> &str {
    req.lines().next().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_config::{StorageConfig, StorageTenantConfig};
    use std::collections::BTreeMap;

    fn cfg() -> ArgoConfig {
        let mut tenants = BTreeMap::new();
        tenants.insert(
            "eu-a".to_string(),
            StorageTenantConfig {
                db_url: ":memory:".into(),
                region: "eu-west-1".into(),
            },
        );
        let mut cfg = ArgoConfig::default();
        cfg.storage = StorageConfig { tenants };
        cfg
    }

    #[tokio::test]
    async fn matching_region_allows_request() {
        let req = "GET /api/v1/audit?tenant=eu-a HTTP/1.1\r\n\r\n";
        assert!(
            reject_cross_region_with_config(req, None, &cfg(), "eu-west-1")
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn mismatched_region_returns_451_and_audit_row() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let req = "GET /api/v1/audit?tenant=eu-a HTTP/1.1\r\n\r\n";
        let (status, body) =
            reject_cross_region_with_config(req, Some(&store), &cfg(), "us-east-1")
                .await
                .unwrap();
        assert_eq!(status, "451 Unavailable For Legal Reasons");
        assert!(body.contains("cross_region_blocked"));
        let kind: String =
            sqlx::query_scalar("SELECT kind FROM audit_record ORDER BY seq DESC LIMIT 1")
                .fetch_one(store.pool())
                .await
                .unwrap();
        assert_eq!(kind, "security.cross_region_blocked");
    }
}
