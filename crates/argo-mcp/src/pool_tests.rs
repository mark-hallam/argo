//! Unit tests for the [`crate::pool`] cache-management primitives.
//!
//! These exercise the parts of the pool that don't need a real MCP
//! subprocess — config defaults, key equality, empty-pool stats, and
//! the `max_clients = 0` edge. Acceptance-criteria tests that need an
//! actual `McpClient` (LRU eviction, idle-TTL reap, per-tenant
//! isolation, acquire→drop→re-acquire) live in `tests/pool.rs` because
//! `env!("CARGO_BIN_EXE_argo-test-mcp-server-echo")` is only set for
//! integration tests; src-side unit tests can't spawn the echo fixture
//! that the pool tests depend on.
use crate::pool::{
    McpClientPool, PoolConfig, PoolKey, PoolStats, DEFAULT_POOL_IDLE_TTL_SECS,
    DEFAULT_POOL_MAX_CLIENTS,
};
use argo_types::TenantId;
use std::time::Duration;

#[test]
fn pool_config_defaults_match_brief() {
    let cfg = PoolConfig::default();
    assert_eq!(cfg.max_clients, DEFAULT_POOL_MAX_CLIENTS);
    assert_eq!(cfg.idle_ttl_secs, DEFAULT_POOL_IDLE_TTL_SECS);
    assert_eq!(DEFAULT_POOL_MAX_CLIENTS, 32);
    assert_eq!(DEFAULT_POOL_IDLE_TTL_SECS, 600);
}

#[test]
fn pool_key_equality_isolates_tenant_and_manifest() {
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let key_a_gmail = PoolKey {
        tenant_id: tenant_a,
        manifest_id: "gmail".into(),
    };
    let key_a_gmail_2 = PoolKey {
        tenant_id: tenant_a,
        manifest_id: "gmail".into(),
    };
    let key_b_gmail = PoolKey {
        tenant_id: tenant_b,
        manifest_id: "gmail".into(),
    };
    let key_a_slack = PoolKey {
        tenant_id: tenant_a,
        manifest_id: "slack".into(),
    };
    assert_eq!(key_a_gmail, key_a_gmail_2);
    assert_ne!(key_a_gmail, key_b_gmail, "different tenants must not collide");
    assert_ne!(key_a_gmail, key_a_slack, "different manifests must not collide");
}

#[test]
fn empty_pool_stats_are_zero() {
    let pool = McpClientPool::new(PoolConfig::default());
    let stats = pool.stats();
    assert_eq!(
        stats,
        PoolStats {
            entries: 0,
            idle_seconds_p50: 0,
            idle_seconds_max: 0,
        }
    );
}

#[tokio::test]
async fn evict_idle_on_empty_pool_is_noop() {
    let pool = McpClientPool::new(PoolConfig {
        max_clients: 4,
        idle_ttl_secs: 1,
    });
    pool.evict_idle().await;
    assert_eq!(pool.stats().entries, 0);
}

#[test]
fn pool_config_max_clients_zero_is_treated_as_one() {
    // Defends against an operator setting `max_clients = 0` in TOML;
    // the LRU cache requires a non-zero capacity. We clamp upward
    // rather than panic so a misconfiguration degrades to a one-slot
    // pool — slow but correct.
    let pool = McpClientPool::new(PoolConfig {
        max_clients: 0,
        idle_ttl_secs: 1,
    });
    assert_eq!(pool.stats().entries, 0);
    // `new_with_idle_ttl(0, _)` exercises the same clamp.
    let pool2 = McpClientPool::new_with_idle_ttl(0, Duration::from_secs(1));
    assert_eq!(pool2.stats().entries, 0);
}
