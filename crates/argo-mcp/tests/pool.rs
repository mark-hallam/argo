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
use std::sync::atomic::{AtomicUsize, Ordering};
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

// mission-mcp-pool-wire-dispatch acceptance smokes
// ----------------------------------------------------------------
// These two tests pin the invariants the dispatch-site wire-up was
// authored to satisfy. They live alongside the pool unit tests because
// the pool is the structural defence — every dispatch site delegates
// its cold-spawn replacement to `pool.acquire(...)` and inherits these
// guarantees by construction. Wiring regressions would surface here as
// well as in the per-handler crate tests.

#[tokio::test]
async fn three_tool_calls_spawn_one_subprocess() {
    // A 3-tool turn against the same (tenant, manifest) must reuse one
    // warm subprocess. Each acquire→drop cycle simulates one tool call;
    // `pool.stats().entries == 1` is the wire-up's warm-reuse contract.
    let pool = McpClientPool::new(PoolConfig::default());
    let tenant = TenantId::new();
    let spawn_count = AtomicUsize::new(0);
    for _ in 0..3 {
        let guard = pool
            .acquire(tenant, "echo", || {
                spawn_count.fetch_add(1, Ordering::SeqCst);
                spawn_echo()
            })
            .await
            .expect("acquire");
        let _ = guard
            .call_tool("echo", serde_json::json!({"text": "tick"}))
            .await
            .expect("call_tool");
    }
    assert_eq!(
        spawn_count.load(Ordering::SeqCst),
        1,
        "first acquire spawns; subsequent two must hit the warm cache"
    );
    assert_eq!(pool.stats().entries, 1);
}

#[tokio::test]
async fn three_tenants_spawn_three_subprocesses() {
    // Per-tenant isolation: three distinct tenants asking for the same
    // `manifest_id` must each receive a distinct subprocess. This is
    // the wire-up's hard correctness invariant; collapsing across
    // tenants would leak credentials.
    let pool = McpClientPool::new(PoolConfig::default());
    let tenants = [TenantId::new(), TenantId::new(), TenantId::new()];
    let mut arcs = Vec::with_capacity(3);
    for t in tenants {
        let guard = pool
            .acquire(t, "shared-manifest", spawn_echo)
            .await
            .expect("acquire");
        arcs.push(guard.client_arc());
    }
    for i in 0..arcs.len() {
        for j in (i + 1)..arcs.len() {
            assert!(
                !Arc::ptr_eq(&arcs[i], &arcs[j]),
                "tenants {i} and {j} must hold distinct subprocesses"
            );
        }
    }
    assert_eq!(pool.stats().entries, 3);
}

#[tokio::test]
async fn max_clients_one_acts_as_disabled_pool_opt_out() {
    // Operators can opt out of cross-turn warmth by setting
    // `max_clients = 1`: each new key evicts the previous, so the
    // observable pre-brief behaviour (one cold spawn per call when
    // the keys differ) is recovered. The internal `.max(1)` clamp in
    // `McpClientPool::new` makes `max_clients = 0` equivalent.
    let pool = McpClientPool::new(PoolConfig {
        max_clients: 1,
        idle_ttl_secs: 600,
    });
    let tenant = TenantId::new();
    let g_a = pool
        .acquire(tenant, "a", spawn_echo)
        .await
        .expect("acquire a");
    let a_arc = g_a.client_arc();
    drop(g_a);
    assert_eq!(pool.stats().entries, 1);
    // Different manifest_id under the same tenant must evict `a`.
    let g_b = pool
        .acquire(tenant, "b", spawn_echo)
        .await
        .expect("acquire b");
    drop(g_b);
    assert_eq!(pool.stats().entries, 1, "max_clients = 1 enforces opt-out");
    let g_a2 = pool
        .acquire(tenant, "a", spawn_echo)
        .await
        .expect("re-acquire a after eviction");
    let a2_arc = g_a2.client_arc();
    assert!(
        !Arc::ptr_eq(&a_arc, &a2_arc),
        "evicted entry must spawn a fresh client under max_clients = 1"
    );
    drop(g_a2);
    // Reap evicted subprocess shutdowns before the test exits.
    tokio::time::sleep(Duration::from_millis(200)).await;
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
