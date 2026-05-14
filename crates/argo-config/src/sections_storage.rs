//! `[storage]` config for per-tenant database residency routing.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `[storage]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    /// `[storage.tenants.<tenant_id>]` entries keyed by tenant id.
    pub tenants: BTreeMap<String, StorageTenantConfig>,
}

/// One `[storage.tenants.<tenant_id>]` entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct StorageTenantConfig {
    /// SQLite database URL/path for this tenant.
    pub db_url: String,
    /// Deployment region that is legally allowed to serve this tenant.
    pub region: String,
}

#[cfg(test)]
mod tests {
    use crate::ArgoConfig;

    #[test]
    fn parses_storage_tenants_map() {
        let cfg: ArgoConfig = toml::from_str(
            r#"
            [storage.tenants.eu-a]
            db_url = "/tmp/eu-a.db"
            region = "eu-west-1"
            "#,
        )
        .unwrap();
        let tenant = cfg.storage.tenants.get("eu-a").unwrap();
        assert_eq!(tenant.db_url, "/tmp/eu-a.db");
        assert_eq!(tenant.region, "eu-west-1");
    }
}
