//! Integration tests for [`argo_mcp::pool`].
//!
//! Lives under `tests/` (not `src/pool_tests.rs` as the brief
//! originally suggested) because pool tests need a real
//! [`McpClient`] subprocess via the
//! `argo-test-mcp-server-echo` binary, and the
//! `CARGO_BIN_EXE_<name>` env var that locates it is set only for
//! integration tests / benchmarks — never for unit tests in `src/`.
//! The pool's surface (acquire / drop / evict / stats) is fully
//! exercised here against a real subprocess.
use argo_mcp::{McpClient, McpClientPool, PoolConfig};
use argo_types::TenantId;
use std::sync::Arc;
use std::time::Duration;

fn server_path() -> &'static str {
    env!("CARGO_BIN_EXE_argo-test-mcp-server-echo")
}

fn spawn_echo() -> impl std::future::Future<Output = argo_mcp::McpResult<McpClient>> {
    McpClient::connect_stdio(server_path(), &[])
}

#[tokio::test]
async fn acquire_lazily_spawns_and_caches() {
    let pool = McpClientPool::new(PoolConfig::default());
    assert_eq!(pool.stats().entries, 0);
    let tenant = TenantId::new();
    let g = pool
        .acquire(tenant, "echo", spawn_echo)
        .await
        .expect("acquire");
    assert_eq!(pool.stats().entries, 1);
    // The guard must Deref to a usable client.
    let tools = g.list_tools().await.expect("list_tools");
    assert!(tools.iter().any(|t| t.name == "echo"));
}

#[tokio::test]
async fn acquire_drop_reacquire_returns_same_instance() {
    let pool = McpClientPool::new(PoolConfig::default());
    let tenant = TenantId::new();
    let first = pool
        .acquire(tenant, "echo", spawn_echo)
        .await
        .expect("first acquire");
    let first_arc = first.client_arc();
    drop(first);
    let second = pool
        .acquire(tenant, "echo", || async {
            panic!("spawn must not run on cache hit");
        })
        .await
        .expect("second acquire");
    let second_arc = second.client_arc();
    assert!(
        Arc::ptr_eq(&first_arc, &second_arc),
        "re-acquire must hand back the cached instance"
    );
}

#[tokio::test]
async fn lru_evicts_oldest_when_over_capacity() {
    let pool = McpClientPool::new(PoolConfig {
        max_clients: 2,
        idle_ttl_secs: 600,
    });
    let tenant = TenantId::new();
    let g_a = pool
        .acquire(tenant, "a", spawn_echo)
        .await
        .expect("acquire a");
    let a_arc = g_a.client_arc();
    drop(g_a);
    let g_b = pool
        .acquire(tenant, "b", spawn_echo)
        .await
        .expect("acquire b");
    drop(g_b);
    assert_eq!(pool.stats().entries, 2);
    // Touch `b` so `a` becomes the LRU candidate, then admit `c`.
    let _ = pool
        .acquire(tenant, "b", || async {
            panic!("b should be cached");
        })
        .await
        .expect("re-acquire b");
    let g_c = pool
        .acquire(tenant, "c", spawn_echo)
        .await
        .expect("acquire c");
    drop(g_c);
    assert_eq!(pool.stats().entries, 2, "max_clients enforced");
    // `a` is gone from the cache: re-acquiring forces a new spawn.
    let g_a2 = pool
        .acquire(tenant, "a", spawn_echo)
        .await
        .expect("re-acquire a after eviction");
    let a2_arc = g_a2.client_arc();
    assert!(
        !Arc::ptr_eq(&a_arc, &a2_arc),
        "evicted entry must spawn a fresh client on next acquire"
    );
    // The evicted client's subprocess is reaped on a background
    // task. Give it a moment to finish so the test exit is clean.
    tokio::time::sleep(Duration::from_millis(200)).await;
}

#[tokio::test]
async fn evict_idle_drops_stale_entries() {
    // Sub-second TTL so the test runs in <1s.
    let pool = McpClientPool::new_with_idle_ttl(8, Duration::from_millis(80));
    let tenant = TenantId::new();
    let g = pool
        .acquire(tenant, "echo", spawn_echo)
        .await
        .expect("acquire");
    drop(g);
    assert_eq!(pool.stats().entries, 1);
    // Sleep past the TTL, then reap.
    tokio::time::sleep(Duration::from_millis(150)).await;
    pool.evict_idle().await;
    assert_eq!(pool.stats().entries, 0, "idle entry must be reaped");
    // Calling evict_idle again on an empty pool is a no-op.
    pool.evict_idle().await;
    assert_eq!(pool.stats().entries, 0);
}

#[tokio::test]
async fn evict_idle_skips_fresh_entries() {
    let pool = McpClientPool::new_with_idle_ttl(8, Duration::from_secs(60));
    let tenant = TenantId::new();
    let _g = pool
        .acquire(tenant, "echo", spawn_echo)
        .await
        .expect("acquire");
    pool.evict_idle().await;
    assert_eq!(pool.stats().entries, 1, "fresh entry must survive reap");
}

#[tokio::test]
async fn per_tenant_isolation() {
    let pool = McpClientPool::new(PoolConfig::default());
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    assert_ne!(tenant_a, tenant_b);
    let g_a = pool
        .acquire(tenant_a, "shared-manifest", spawn_echo)
        .await
        .expect("tenant_a acquire");
    let a_arc = g_a.client_arc();
    let g_b = pool
        .acquire(tenant_b, "shared-manifest", spawn_echo)
        .await
        .expect("tenant_b acquire");
    let b_arc = g_b.client_arc();
    assert!(
        !Arc::ptr_eq(&a_arc, &b_arc),
        "same manifest_id under different tenants must spawn distinct clients"
    );
    assert_eq!(pool.stats().entries, 2);
}

#[tokio::test]
async fn pool_guard_drop_does_not_shutdown_client() {
    let pool = McpClientPool::new(PoolConfig::default());
    let tenant = TenantId::new();
    let g1 = pool
        .acquire(tenant, "echo", spawn_echo)
        .await
        .expect("acquire");
    drop(g1);
    // If guard-drop shut the client down, the next acquire's
    // call_tool would surface `McpError::Shutdown`. It must not.
    let g2 = pool
        .acquire(tenant, "echo", || async {
            panic!("must be cached");
        })
        .await
        .expect("re-acquire");
    let r = g2
        .call_tool("echo", serde_json::json!({"text": "still alive"}))
        .await
        .expect("call_tool");
    assert!(!r.is_error);
}

#[tokio::test]
async fn stats_reports_entries_and_idle_window() {
    let pool = McpClientPool::new(PoolConfig::default());
    let s0 = pool.stats();
    assert_eq!(s0.entries, 0);
    assert_eq!(s0.idle_seconds_p50, 0);
    assert_eq!(s0.idle_seconds_max, 0);
    let tenant = TenantId::new();
    let _g = pool
        .acquire(tenant, "echo", spawn_echo)
        .await
        .expect("acquire");
    let s1 = pool.stats();
    assert_eq!(s1.entries, 1);
    // Just-acquired entries report 0 idle seconds.
    assert_eq!(s1.idle_seconds_max, 0);
    assert_eq!(s1.idle_seconds_p50, 0);
}
