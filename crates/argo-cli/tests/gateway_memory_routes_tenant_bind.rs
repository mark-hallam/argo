//! Store-level regression coverage for the gateway memory route
//! tenant bind. The route handlers are structurally pinned in
//! `gateway_uuid_routes_tenant_ownership.rs`; these tests exercise
//! the tenant-aware MemoryStore methods those handlers call.

use argo_memory::{MemoryFile, MemoryStore, PrincipalOverride};
use argo_store::SessionStore;
use argo_types::{Lock, TenantId, WorkspaceId};
use std::sync::Arc;

async fn fresh_store() -> (
    Arc<SessionStore>,
    MemoryStore,
    TenantId,
    TenantId,
    WorkspaceId,
) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open db"));
    store.migrate().await.expect("migrate");
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let workspace = seed_workspace(&store, tenant_a, "tenant-a").await;
    (
        Arc::clone(&store),
        MemoryStore::new(store),
        tenant_a,
        tenant_b,
        workspace,
    )
}

async fn seed_workspace(store: &SessionStore, tenant: TenantId, name: &str) -> WorkspaceId {
    let workspace = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, ?, '.', strftime('%s','now'))",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(name)
    .execute(store.pool())
    .await
    .expect("seed workspace");
    workspace
}

#[tokio::test]
async fn same_tenant_lock_and_get_succeed() {
    let (_store, memory, tenant_a, tenant_b, workspace) = fresh_store().await;
    let entry = memory
        .append(workspace, MemoryFile::Memory, "remember this".into(), None)
        .await
        .expect("append memory");

    let changed = memory
        .set_lock_for_tenant(
            entry.id,
            &tenant_a.0.to_string(),
            Lock::UserProtected,
            PrincipalOverride::Force,
        )
        .await
        .expect("same-tenant lock");
    assert!(changed, "same-tenant lock should mutate");

    let own = memory
        .get_for_tenant(entry.id, &tenant_a.0.to_string())
        .await
        .expect("same-tenant get")
        .expect("entry visible to owning tenant");
    assert_eq!(own.lock, Lock::UserProtected);
    assert!(
        memory
            .get_for_tenant(entry.id, &tenant_b.0.to_string())
            .await
            .expect("cross-tenant get")
            .is_none(),
        "non-owner tenant must not see the entry"
    );
}

#[tokio::test]
async fn same_tenant_supersede_and_versions_succeed() {
    let (_store, memory, tenant_a, _tenant_b, workspace) = fresh_store().await;
    let first = memory
        .append(workspace, MemoryFile::User, "v1".into(), None)
        .await
        .expect("append memory");

    let second = memory
        .supersede_for_tenant(
            first.id,
            &tenant_a.0.to_string(),
            "v2".into(),
            None,
            PrincipalOverride::Force,
        )
        .await
        .expect("same-tenant supersede")
        .expect("entry visible to owning tenant");

    let versions = memory
        .versions_for_entry_for_tenant(first.id, &tenant_a.0.to_string())
        .await
        .expect("same-tenant versions")
        .expect("version chain visible");
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].body, "v1");
    assert_eq!(versions[1].body, "v2");
    assert_eq!(versions[1].id, second.id);
}

#[tokio::test]
async fn cross_tenant_mutations_are_refused_without_side_effects() {
    let (_store, memory, _tenant_a, tenant_b, workspace) = fresh_store().await;
    let entry = memory
        .append(workspace, MemoryFile::Soul, "original".into(), None)
        .await
        .expect("append memory");
    let tenant_b = tenant_b.0.to_string();

    let locked = memory
        .set_lock_for_tenant(
            entry.id,
            &tenant_b,
            Lock::ContentLocked,
            PrincipalOverride::Force,
        )
        .await
        .expect("cross-tenant lock");
    assert!(!locked, "cross-tenant lock should be refused");
    assert_eq!(memory.get(entry.id).await.expect("get").lock, Lock::None);

    let edited = memory
        .supersede_for_tenant(
            entry.id,
            &tenant_b,
            "tenant-b edit".into(),
            None,
            PrincipalOverride::Force,
        )
        .await
        .expect("cross-tenant supersede");
    assert!(edited.is_none(), "cross-tenant edit should be hidden");

    let reverted = memory
        .revert_for_tenant(entry.id, &tenant_b, PrincipalOverride::Force)
        .await
        .expect("cross-tenant revert");
    assert!(reverted.is_none(), "cross-tenant revert should be hidden");

    let versions = memory.versions_for_entry(entry.id).await.expect("versions");
    assert_eq!(versions.len(), 1, "no cross-tenant write should persist");
    assert!(
        memory
            .versions_for_entry_for_tenant(entry.id, &tenant_b)
            .await
            .expect("cross-tenant versions")
            .is_none(),
        "non-owner tenant must not see the version chain"
    );
}

#[tokio::test]
async fn row_level_tenant_mismatch_fails_closed() {
    let (store, memory, tenant_a, tenant_b, workspace) = fresh_store().await;
    let entry = memory
        .append(workspace, MemoryFile::Memory, "tenant tagged".into(), None)
        .await
        .expect("append memory");

    sqlx::query("UPDATE memory_entry SET tenant_id = ? WHERE id = ?")
        .bind(tenant_b.0.as_bytes().as_slice())
        .bind(entry.id.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .expect("tag row with conflicting tenant");

    assert!(
        memory
            .get_for_tenant(entry.id, &tenant_a.0.to_string())
            .await
            .expect("get_for_tenant")
            .is_none(),
        "explicit memory_entry.tenant_id mismatch must not fall back \
         to workspace ownership"
    );
    assert!(
        !memory
            .memory_belongs_to_tenant(entry.id, &tenant_a.0.to_string())
            .await
            .expect("memory_belongs_to_tenant"),
        "mutation preflight must fail closed on row/workspace tenant mismatch"
    );
}

#[tokio::test]
async fn versions_skip_row_level_tenant_mismatches() {
    let (store, memory, tenant_a, tenant_b, workspace) = fresh_store().await;
    let first = memory
        .append(workspace, MemoryFile::Memory, "tenant-a v1".into(), None)
        .await
        .expect("append first");
    let second = memory
        .supersede_for_tenant(
            first.id,
            &tenant_a.0.to_string(),
            "tenant-a v2".into(),
            None,
            argo_memory::PrincipalOverride::Force,
        )
        .await
        .expect("supersede")
        .expect("same tenant can supersede");

    sqlx::query("UPDATE memory_entry SET tenant_id = ? WHERE id = ?")
        .bind(tenant_b.0.as_bytes().as_slice())
        .bind(second.id.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .expect("tag sibling row with conflicting tenant");

    let versions = memory
        .versions_for_entry_for_tenant(first.id, &tenant_a.0.to_string())
        .await
        .expect("versions_for_entry_for_tenant")
        .expect("first entry belongs to tenant_a");

    assert_eq!(
        versions.iter().map(|entry| entry.id).collect::<Vec<_>>(),
        vec![first.id],
        "version chains must filter every returned row by tenant"
    );
}
