//! Per-tenant `SessionStore` registry for data-residency routing.
use crate::wal_tuning::WalTuningProfile;
use crate::{SessionStore, StoreResult};
use argo_types::ArgoError;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Config needed to open one tenant-specific store.
///
/// `db_key_hex` is the per-tenant SQLCipher key (64-char lowercase hex,
/// produced by `argo_store::encryption::base64_to_hex` from the
/// `SecretString` that `argo_config::cred::db_key::get_or_create_db_key`
/// returns). Each tenant MUST receive a distinct key — sharing one key
/// across tenants would be a cross-tenant data-confidentiality
/// regression
/// (`mission-sqlcipher-4-default-flip` brief, hidden-constraints §
/// "Per-tenant keys are non-negotiable"). A `None` value disables
/// encryption for that tenant; the only legitimate use is BYOK-disabled
/// internal-fixture deployments where the operator has explicitly
/// opted out, and even then `SessionStore::open` will still consult
/// `SKYCOT_DB_KEY_BASE64` from env before falling through to
/// plaintext.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantPoolConfig {
    /// Tenant id that resolves to this pool.
    pub tenant_id: String,
    /// SQLite database URL/path.
    pub db_url: String,
    /// Legal deployment region for this tenant.
    pub region: String,
    /// Per-tenant SQLCipher key, hex-encoded (64 chars). When `None`,
    /// the tenant pool falls back to the env-driven default — see the
    /// struct-level doc.
    pub db_key_hex: Option<String>,
}

/// Resolved store plus routing metadata.
#[derive(Clone)]
pub struct ResolvedTenantStore {
    /// Tenant id requested by the caller.
    pub tenant_id: String,
    /// Region associated with the chosen pool.
    pub region: Option<String>,
    /// Store selected for this request.
    pub store: Arc<SessionStore>,
    /// Whether the default pool was used because no tenant override exists.
    pub used_default: bool,
}

/// Tenant health row for operator CLI output.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TenantPoolStatus {
    /// Tenant id.
    pub tenant_id: String,
    /// Configured region.
    pub region: String,
    /// Configured SQLite URL/path.
    pub db_url: String,
    /// Current schema version, if readable.
    pub schema_version: Option<u32>,
    /// True when schema version could be read.
    pub healthy: bool,
    /// Error string when health could not be read.
    pub error: Option<String>,
}

struct TenantPool {
    db_url: String,
    region: String,
    store: Arc<SessionStore>,
}

/// Registry routing tenant ids to tenant-specific SQLite pools.
pub struct MultiTenantStore {
    default_pool: Arc<SessionStore>,
    tenants: BTreeMap<String, TenantPool>,
}

impl MultiTenantStore {
    /// Build a registry from an already-open default store and tenant DB configs.
    ///
    /// Each tenant store is opened with the per-tenant SQLCipher key
    /// from [`TenantPoolConfig::db_key_hex`] when supplied — the v1
    /// encrypted-by-default posture
    /// (`mission-sqlcipher-4-default-flip`). Callers should populate
    /// `db_key_hex` via
    /// `argo_config::cred::db_key::get_or_create_db_key(tenant_id)` →
    /// `argo_store::encryption::base64_to_hex` before passing the
    /// config in. A `None` value falls back to `SessionStore::open`,
    /// which still honors `SKYCOT_DB_KEY_BASE64` from env.
    pub async fn open(
        default_pool: SessionStore,
        tenants: impl IntoIterator<Item = TenantPoolConfig>,
    ) -> StoreResult<Self> {
        let mut pools = BTreeMap::new();
        for cfg in tenants {
            validate_config(&cfg)?;
            let store = match cfg.db_key_hex.as_deref() {
                Some(hex) => {
                    SessionStore::open_with_hex_key(
                        &cfg.db_url,
                        WalTuningProfile::default(),
                        hex,
                    )
                    .await?
                }
                None => SessionStore::open(&cfg.db_url).await?,
            };
            pools.insert(
                cfg.tenant_id,
                TenantPool {
                    db_url: cfg.db_url,
                    region: cfg.region,
                    store: Arc::new(store),
                },
            );
        }
        Ok(Self {
            default_pool: Arc::new(default_pool),
            tenants: pools,
        })
    }

    /// Resolve a tenant to its configured pool, falling back to default.
    #[must_use]
    pub fn resolve(&self, tenant_id: &str) -> ResolvedTenantStore {
        if let Some(pool) = self.tenants.get(tenant_id) {
            return ResolvedTenantStore {
                tenant_id: tenant_id.to_string(),
                region: Some(pool.region.clone()),
                store: Arc::clone(&pool.store),
                used_default: false,
            };
        }
        ResolvedTenantStore {
            tenant_id: tenant_id.to_string(),
            region: None,
            store: Arc::clone(&self.default_pool),
            used_default: true,
        }
    }

    /// Run migrations for one configured tenant. Unknown tenants use default.
    pub async fn migrate_tenant(&self, tenant_id: &str) -> StoreResult<u32> {
        let resolved = self.resolve(tenant_id);
        resolved.store.migrate().await?;
        resolved.store.schema_version().await
    }

    /// Return health rows for every configured tenant.
    pub async fn tenant_statuses(&self) -> Vec<TenantPoolStatus> {
        let mut rows = Vec::with_capacity(self.tenants.len());
        for (tenant_id, pool) in &self.tenants {
            match pool.store.schema_version().await {
                Ok(version) => rows.push(TenantPoolStatus {
                    tenant_id: tenant_id.clone(),
                    region: pool.region.clone(),
                    db_url: pool.db_url.clone(),
                    schema_version: Some(version),
                    healthy: true,
                    error: None,
                }),
                Err(e) => rows.push(TenantPoolStatus {
                    tenant_id: tenant_id.clone(),
                    region: pool.region.clone(),
                    db_url: pool.db_url.clone(),
                    schema_version: None,
                    healthy: false,
                    error: Some(e.to_string()),
                }),
            }
        }
        rows
    }

    /// True when a tenant has an explicit pool.
    #[must_use]
    pub fn has_tenant(&self, tenant_id: &str) -> bool {
        self.tenants.contains_key(tenant_id)
    }
}

fn validate_config(cfg: &TenantPoolConfig) -> StoreResult<()> {
    if cfg.tenant_id.trim().is_empty() {
        return Err(ArgoError::Provider("tenant pool id is empty".into()));
    }
    if cfg.db_url.trim().is_empty() {
        return Err(ArgoError::Provider(format!(
            "tenant {} db_url is empty",
            cfg.tenant_id
        )));
    }
    if cfg.region.trim().is_empty() {
        return Err(ArgoError::Provider(format!(
            "tenant {} region is empty",
            cfg.tenant_id
        )));
    }
    Ok(())
}

#[cfg(test)]
#[path = "multi_tenant_pool_tests.rs"]
mod multi_tenant_pool_tests;
