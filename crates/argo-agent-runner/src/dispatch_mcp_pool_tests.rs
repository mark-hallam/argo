//! Tenant-scoping tests for the connector credential-injection seam.
//!
//! These pin the SECURITY-CRITICAL guarantee of the connector-reach
//! unlock: the env injected into a connector subprocess for tenant A
//! contains ONLY tenant A's stored credentials, never tenant B's.

use super::McpConnectionPool;
use argo_types::TenantId;
use std::collections::BTreeMap;

fn env_map(pool: &McpConnectionPool, tenant: TenantId) -> BTreeMap<String, String> {
    pool.connector_env(tenant).into_iter().collect()
}

#[test]
fn connector_env_injects_only_the_calling_tenants_creds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    // Each tenant stores a connector token in their OWN vault section.
    argo_config::cred::set_tenant(dir.path(), &tenant_a, "SLACK_BOT_TOKEN", "xoxb-a")
        .expect("set tenant_a");
    argo_config::cred::set_tenant(dir.path(), &tenant_b, "SLACK_BOT_TOKEN", "xoxb-b")
        .expect("set tenant_b");

    let pool = McpConnectionPool::with_workspace(dir.path().to_path_buf());

    let env_a = env_map(&pool, tenant_a);
    assert_eq!(
        env_a.get("SLACK_BOT_TOKEN").map(String::as_str),
        Some("xoxb-a"),
        "tenant_a's connector env must carry tenant_a's token"
    );
    // The cross-tenant guarantee: tenant_a NEVER sees tenant_b's value.
    assert_ne!(
        env_a.get("SLACK_BOT_TOKEN").map(String::as_str),
        Some("xoxb-b"),
        "tenant_a's connector env leaked tenant_b's token",
    );
    assert_eq!(
        env_a.get("ARGO_MCP_TENANT_ID").map(String::as_str),
        Some(tenant_a.to_string().as_str()),
        "connector env must tag the spawning tenant id",
    );

    let env_b = env_map(&pool, tenant_b);
    assert_eq!(
        env_b.get("SLACK_BOT_TOKEN").map(String::as_str),
        Some("xoxb-b"),
        "tenant_b's connector env must carry tenant_b's token"
    );
}

#[test]
fn connector_env_without_workspace_injects_only_tenant_id() {
    // Legacy/test path: no workspace → no vault read, just the tenant tag.
    let pool = McpConnectionPool::default();
    let tenant = TenantId::new();
    let env = env_map(&pool, tenant);
    assert_eq!(env.len(), 1, "no-workspace pool must inject only the tenant id");
    assert_eq!(
        env.get("ARGO_MCP_TENANT_ID").map(String::as_str),
        Some(tenant.to_string().as_str()),
    );
}
