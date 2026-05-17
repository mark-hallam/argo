//! Unit tests for per-(tenant, connector) MCP rate limits.
#![cfg(test)]

use super::rate_limit::BucketRegistry;
use super::{McpClientPool, McpError, PoolConfig};
use argo_types::TenantId;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn gmail_default_allows_thirty_then_rejects_thirty_first() {
    let registry = BucketRegistry::new();
    let tenant = TenantId::new();
    for _ in 0..30 {
        registry.check(tenant, "gmail").unwrap();
    }
    let err = registry.check(tenant, "gmail").unwrap_err();
    assert!(matches!(
        err,
        McpError::RateLimited {
            retry_after_ms: 2000,
            limit: 30
        }
    ));
}

#[test]
fn tenant_b_quota_is_independent_of_tenant_a() {
    let registry = BucketRegistry::new();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    for _ in 0..30 {
        registry.check(tenant_a, "gmail").unwrap();
    }
    assert!(registry.check(tenant_a, "gmail").is_err());
    for _ in 0..30 {
        registry.check(tenant_b, "gmail").unwrap();
    }
}

#[test]
fn refill_tick_adds_fractional_tokens() {
    let registry = BucketRegistry::new();
    let tenant = TenantId::new();
    for _ in 0..30 {
        registry.check(tenant, "gmail").unwrap();
    }
    assert!(registry.check(tenant, "gmail").is_err());
    registry.refill_once_for_test();
    assert!(registry.check(tenant, "gmail").is_err());
    registry.refill_once_for_test();
    registry.check(tenant, "gmail").unwrap();
}

#[test]
fn lru_evicts_oldest_pair_at_capacity() {
    let registry = BucketRegistry::with_capacity(2);
    let tenant = TenantId::new();
    registry.check(tenant, "gmail").unwrap();
    registry.check(tenant, "slack").unwrap();
    registry.check(tenant, "stripe").unwrap();
    assert_eq!(registry.len_for_test(), 2);
    assert!(!registry.contains_for_test(tenant, "gmail"));
    assert!(registry.contains_for_test(tenant, "slack"));
    assert!(registry.contains_for_test(tenant, "stripe"));
}

#[test]
fn connector_override_raises_budget() {
    let registry = BucketRegistry::new();
    let tenant = TenantId::new();
    registry.install_overrides([(tenant, "gmail", 100)]);
    for _ in 0..100 {
        registry.check(tenant, "gmail").unwrap();
    }
    let err = registry.check(tenant, "gmail").unwrap_err();
    assert!(matches!(err, McpError::RateLimited { limit: 100, .. }));
}

#[tokio::test]
async fn pool_rejects_before_spawn_when_bucket_exhausted() {
    static SPAWNS: AtomicUsize = AtomicUsize::new(0);
    let pool = McpClientPool::new(PoolConfig::default());
    let tenant = TenantId::new();
    pool.rate_limiter().install_override(tenant, "gmail", 1);
    pool.rate_limiter().check(tenant, "gmail").unwrap();

    let err = pool
        .acquire(tenant, "gmail", || async {
            SPAWNS.fetch_add(1, Ordering::SeqCst);
            unreachable!("rate limit should reject before spawn")
        })
        .await
        .unwrap_err();

    assert!(matches!(err, McpError::RateLimited { limit: 1, .. }));
    assert_eq!(SPAWNS.load(Ordering::SeqCst), 0);
}
