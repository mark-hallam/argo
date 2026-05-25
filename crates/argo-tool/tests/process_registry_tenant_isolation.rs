//! Cross-tenant negative tests for [`ProcessRegistry`].
//!
//! `mission-process-registry-tenant-keyed` adds a per-entry tenant
//! guard so a single shared registry can be wired into many sessions
//! without cross-tenant data leaks. These tests exercise every public
//! method (`spawn`/`list`/`status`/`tail`/`kill`/`reap_finished`/`len`/
//! `is_empty`) from the perspective of an *unrelated* tenant and
//! assert isolation holds.
//!
//! The companion pin test
//! `process_registry_kill_rejected_across_tenants` in
//! `argo_tool_substrate_pin_test.rs` covers the same property at the
//! tool-dispatch layer (`Terminal` + `Process` tools); this file
//! exercises the registry API directly so a regression surfaces even
//! if the tool wrapper changes.
#![cfg(unix)]

use std::sync::Arc;
use std::time::Duration;

use argo_tool::process_registry::{ProcessId, ProcessRegistry, SpawnRequest};
use argo_types::{ArgoError, TenantId};

fn tenant_a() -> TenantId {
    TenantId(uuid::Uuid::from_u128(0xA))
}

fn tenant_b() -> TenantId {
    TenantId(uuid::Uuid::from_u128(0xB))
}

fn ok_dir() -> std::path::PathBuf {
    std::env::temp_dir()
}

async fn spawn_long_lived(reg: &ProcessRegistry, tenant: TenantId) -> ProcessId {
    reg.spawn(
        SpawnRequest {
            command: "sleep 60".into(),
            working_dir: ok_dir(),
        },
        tenant,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn list_returns_only_own_tenants_entries() {
    let reg = Arc::new(ProcessRegistry::new());
    let id_a = spawn_long_lived(&reg, tenant_a()).await;
    let id_b = spawn_long_lived(&reg, tenant_b()).await;

    let live_a = reg.list(tenant_a()).await;
    let live_b = reg.list(tenant_b()).await;
    assert_eq!(live_a.len(), 1, "tenant A sees only its own entry");
    assert_eq!(live_b.len(), 1, "tenant B sees only its own entry");
    assert_eq!(live_a[0].id, id_a);
    assert_eq!(live_b[0].id, id_b);

    // Cleanup.
    reg.kill(&id_a, tenant_a(), Duration::from_secs(2))
        .await
        .unwrap();
    reg.kill(&id_b, tenant_b(), Duration::from_secs(2))
        .await
        .unwrap();
}

#[tokio::test]
async fn status_returns_none_across_tenants() {
    let reg = Arc::new(ProcessRegistry::new());
    let id_a = spawn_long_lived(&reg, tenant_a()).await;

    assert!(
        reg.status(&id_a, tenant_a()).await.is_some(),
        "owning tenant can read status"
    );
    assert!(
        reg.status(&id_a, tenant_b()).await.is_none(),
        "non-owning tenant must see status = None (no existence leak)"
    );

    reg.kill(&id_a, tenant_a(), Duration::from_secs(2))
        .await
        .unwrap();
}

#[tokio::test]
async fn tail_returns_none_across_tenants() {
    let reg = Arc::new(ProcessRegistry::new());
    let id_a = reg
        .spawn(
            SpawnRequest {
                command: "echo tenant-a-secret; sleep 0.2".into(),
                working_dir: ok_dir(),
            },
            tenant_a(),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;

    let owned_tail = reg.tail(&id_a, tenant_a()).await.expect("owner sees tail");
    assert!(
        owned_tail.stdout.contains("tenant-a-secret"),
        "owner reads stdout"
    );
    assert!(
        reg.tail(&id_a, tenant_b()).await.is_none(),
        "non-owning tenant must not see stdout (would leak secret payloads)"
    );

    reg.reap_finished(tenant_a()).await;
}

#[tokio::test]
async fn kill_across_tenants_errors_like_unknown_pid() {
    let reg = Arc::new(ProcessRegistry::new());
    let id_a = spawn_long_lived(&reg, tenant_a()).await;

    let err = reg
        .kill(&id_a, tenant_b(), Duration::from_secs(2))
        .await
        .unwrap_err();
    match err {
        ArgoError::Tool { tool, msg } => {
            assert_eq!(tool, "process_registry");
            assert!(
                msg.contains("no such process"),
                "cross-tenant kill must mirror unknown-pid error \
                 (no distinction = no existence leak). Got: {msg}"
            );
        }
        other => panic!("expected ArgoError::Tool, got: {other:?}"),
    }
    // Tenant A's entry survives.
    assert_eq!(reg.len(tenant_a()).await, 1);

    reg.kill(&id_a, tenant_a(), Duration::from_secs(2))
        .await
        .unwrap();
    assert_eq!(reg.len(tenant_a()).await, 0);
}

#[tokio::test]
async fn reap_finished_only_touches_own_tenant() {
    let reg = Arc::new(ProcessRegistry::new());
    // tenant A: short-lived (will exit immediately).
    let _id_a = reg
        .spawn(
            SpawnRequest {
                command: "true".into(),
                working_dir: ok_dir(),
            },
            tenant_a(),
        )
        .await
        .unwrap();
    // tenant B: short-lived but tenant A's reap must not drop it.
    let _id_b = reg
        .spawn(
            SpawnRequest {
                command: "true".into(),
                working_dir: ok_dir(),
            },
            tenant_b(),
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    reg.reap_finished(tenant_a()).await;
    assert_eq!(
        reg.len(tenant_a()).await,
        0,
        "tenant A's exited entry is reaped"
    );
    assert_eq!(
        reg.len(tenant_b()).await,
        1,
        "tenant B's entry must survive tenant A's reap"
    );

    reg.reap_finished(tenant_b()).await;
    assert_eq!(reg.len(tenant_b()).await, 0);
}

#[tokio::test]
async fn len_and_is_empty_are_per_tenant() {
    let reg = Arc::new(ProcessRegistry::new());
    assert!(reg.is_empty(tenant_a()).await);
    assert!(reg.is_empty(tenant_b()).await);

    let id_a = spawn_long_lived(&reg, tenant_a()).await;
    assert_eq!(reg.len(tenant_a()).await, 1);
    assert_eq!(
        reg.len(tenant_b()).await,
        0,
        "tenant B's count must not move when tenant A spawns"
    );
    assert!(!reg.is_empty(tenant_a()).await);
    assert!(reg.is_empty(tenant_b()).await);

    reg.kill(&id_a, tenant_a(), Duration::from_secs(2))
        .await
        .unwrap();
}
