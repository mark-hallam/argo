//! Contract tests for the MCP client pool's tenant-keyed cache.
//!
//! The pool must key warm clients by `(TenantId, manifest_id)`. A
//! manifest-only cache would reuse subprocesses across tenants and
//! could carry one tenant's connector credentials into another tenant.
use argo_mcp::{McpClient, McpClientPool, PoolConfig, PoolGuard};
use argo_types::TenantId;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn server_path() -> &'static str {
    env!("CARGO_BIN_EXE_argo-test-mcp-server-echo")
}

fn spawn_echo() -> impl Future<Output = argo_mcp::McpResult<McpClient>> {
    McpClient::connect_stdio_with_timeout(server_path(), &[], Duration::from_secs(5))
}

async fn spawn_echo_with_auth_label(
    seen_auth_labels: Arc<Mutex<Vec<&'static str>>>,
    auth_label: &'static str,
) -> argo_mcp::McpResult<McpClient> {
    seen_auth_labels
        .lock()
        .expect("labels lock")
        .push(auth_label);
    spawn_echo().await
}

fn acquire_signature_requires_tenant_id<'a, F, Fut>(
    pool: &'a Arc<McpClientPool>,
    tenant_id: TenantId,
    manifest_id: &'a str,
    spawn: F,
) -> impl Future<Output = argo_mcp::McpResult<PoolGuard>> + 'a
where
    F: FnOnce() -> Fut + 'a,
    Fut: Future<Output = argo_mcp::McpResult<McpClient>> + 'a,
{
    pool.acquire(tenant_id, manifest_id, spawn)
}

#[tokio::test]
async fn two_tenants_same_manifest_receive_distinct_clients() {
    let pool = McpClientPool::new(PoolConfig::default());
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    assert_ne!(tenant_a, tenant_b);

    let guard_a = pool
        .acquire(tenant_a, "shared-manifest", spawn_echo)
        .await
        .expect("tenant a acquire");
    let guard_b = pool
        .acquire(tenant_b, "shared-manifest", spawn_echo)
        .await
        .expect("tenant b acquire");

    let client_a = guard_a.client_arc();
    let client_b = guard_b.client_arc();
    assert!(
        !Arc::ptr_eq(&client_a, &client_b),
        "same manifest under different tenants must not reuse a client"
    );
    assert_eq!(pool.stats().entries, 2);
}

#[tokio::test]
async fn same_tenant_same_manifest_reuses_expected_client() {
    let pool = McpClientPool::new(PoolConfig::default());
    let tenant = TenantId::new();

    let first = pool
        .acquire(tenant, "shared-manifest", spawn_echo)
        .await
        .expect("first acquire");
    let first_client = first.client_arc();
    drop(first);

    let second = pool
        .acquire(tenant, "shared-manifest", || async {
            panic!("cache hit must not invoke the spawn factory");
        })
        .await
        .expect("second acquire");
    let second_client = second.client_arc();

    assert!(
        Arc::ptr_eq(&first_client, &second_client),
        "same tenant and manifest must reuse the cached client"
    );
    assert_eq!(pool.stats().entries, 1);
}

#[tokio::test]
async fn factory_supplied_auth_labels_do_not_cross_tenant_boundaries() {
    let pool = McpClientPool::new(PoolConfig::default());
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let seen_auth_labels = Arc::new(Mutex::new(Vec::new()));

    let guard_a = pool
        .acquire(tenant_a, "gmail", {
            let seen_auth_labels = Arc::clone(&seen_auth_labels);
            move || spawn_echo_with_auth_label(seen_auth_labels, "tenant-a-oauth")
        })
        .await
        .expect("tenant a acquire");
    let client_a = guard_a.client_arc();
    drop(guard_a);

    let guard_b = pool
        .acquire(tenant_b, "gmail", {
            let seen_auth_labels = Arc::clone(&seen_auth_labels);
            move || spawn_echo_with_auth_label(seen_auth_labels, "tenant-b-oauth")
        })
        .await
        .expect("tenant b acquire");
    let client_b = guard_b.client_arc();

    assert!(
        !Arc::ptr_eq(&client_a, &client_b),
        "tenant b must not receive tenant a's credential-bearing client"
    );
    assert_eq!(
        seen_auth_labels.lock().expect("labels lock").as_slice(),
        ["tenant-a-oauth", "tenant-b-oauth"],
        "each tenant must invoke its own credential factory"
    );

    let guard_a_again = pool
        .acquire(tenant_a, "gmail", || async {
            panic!("same tenant should reuse its own cached credential client");
        })
        .await
        .expect("tenant a reacquire");
    let client_a_again = guard_a_again.client_arc();
    assert!(
        Arc::ptr_eq(&client_a, &client_a_again),
        "tenant a should reacquire tenant a's original client"
    );
    assert_eq!(pool.stats().entries, 2);
}

#[test]
fn acquire_signature_drift_sentinel_keeps_tenant_parameter() {
    let pool = McpClientPool::new(PoolConfig::default());
    let future = acquire_signature_requires_tenant_id(
        &pool,
        TenantId::new(),
        "signature-sentinel",
        spawn_echo,
    );
    drop(future);
}
