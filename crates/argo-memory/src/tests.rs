use super::*;
use argo_store::SessionStore;
use argo_types::{Lock, TenantId, WorkspaceId};
use std::sync::Arc;

async fn fresh() -> (MemoryStore, WorkspaceId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    (MemoryStore::new(Arc::new(store)), ws)
}

#[tokio::test]
async fn append_then_list_active() {
    let (m, ws) = fresh().await;
    let entry = m
        .append(ws, MemoryFile::Memory, "first".into(), None)
        .await
        .unwrap();
    assert_eq!(entry.body, "first");
    assert_eq!(entry.lock, Lock::None);
    let active = m.list_active(ws, MemoryFile::Memory).await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, entry.id);
}

#[tokio::test]
async fn supersede_marks_old_inactive() {
    let (m, ws) = fresh().await;
    let first = m
        .append(ws, MemoryFile::Memory, "v1".into(), None)
        .await
        .unwrap();
    let second = m
        .supersede(first.id, "v2".into(), None, PrincipalOverride::Agent)
        .await
        .unwrap();
    let active = m.list_active(ws, MemoryFile::Memory).await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, second.id);
    let prev = m.get(first.id).await.unwrap();
    assert_eq!(prev.superseded_by, Some(second.id));
}

#[tokio::test]
async fn user_protected_blocks_agent_supersede() {
    let (m, ws) = fresh().await;
    let first = m
        .append(ws, MemoryFile::User, "core profile".into(), None)
        .await
        .unwrap();
    m.set_lock(first.id, Lock::UserProtected, PrincipalOverride::Force)
        .await
        .unwrap();
    let err = m
        .supersede(
            first.id,
            "agent rewrite".into(),
            None,
            PrincipalOverride::Agent,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::UserProtected { .. }));
    // No entry was persisted by the failed write.
    let active = m.list_active(ws, MemoryFile::User).await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, first.id);
}

#[tokio::test]
async fn user_protected_allows_force_override() {
    let (m, ws) = fresh().await;
    let first = m
        .append(ws, MemoryFile::User, "core profile".into(), None)
        .await
        .unwrap();
    m.set_lock(first.id, Lock::UserProtected, PrincipalOverride::Force)
        .await
        .unwrap();
    let second = m
        .supersede(
            first.id,
            "user rewrite".into(),
            None,
            PrincipalOverride::Force,
        )
        .await
        .unwrap();
    assert_ne!(first.id, second.id);
    let active = m.list_active(ws, MemoryFile::User).await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].body, "user rewrite");
}

#[tokio::test]
async fn revert_writes_a_new_entry_with_old_body() {
    let (m, ws) = fresh().await;
    let v1 = m
        .append(ws, MemoryFile::Memory, "v1".into(), None)
        .await
        .unwrap();
    let v2 = m
        .supersede(v1.id, "v2".into(), None, PrincipalOverride::Agent)
        .await
        .unwrap();
    let _ = m.revert(v1.id, PrincipalOverride::Agent).await.unwrap();
    let active = m.list_active(ws, MemoryFile::Memory).await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].body, "v1");
    // The reverted-from head is now superseded.
    let head_after = m.get(v2.id).await.unwrap();
    assert!(head_after.superseded_by.is_some());
}

#[tokio::test]
async fn agents_cannot_set_or_clear_locks() {
    let (m, ws) = fresh().await;
    let entry = m
        .append(ws, MemoryFile::User, "anything".into(), None)
        .await
        .unwrap();
    let err = m
        .set_lock(entry.id, Lock::UserProtected, PrincipalOverride::Agent)
        .await
        .unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::UserProtected { .. }));
}

#[tokio::test]
async fn lock_round_trips_via_list_active() {
    let (m, ws) = fresh().await;
    let entry = m
        .append(ws, MemoryFile::Soul, "voice".into(), None)
        .await
        .unwrap();
    m.set_lock(entry.id, Lock::ContentLocked, PrincipalOverride::Force)
        .await
        .unwrap();
    let active = m.list_active(ws, MemoryFile::Soul).await.unwrap();
    assert_eq!(active[0].lock, Lock::ContentLocked);
}

#[tokio::test]
async fn hash_is_sha256_of_body() {
    let (m, ws) = fresh().await;
    let body = "stable input";
    let entry = m
        .append(ws, MemoryFile::Memory, body.into(), None)
        .await
        .unwrap();
    let mut h = sha2::Sha256::new();
    use sha2::Digest;
    h.update(body.as_bytes());
    let want = h.finalize();
    assert_eq!(entry.hash, want.as_slice());
}

// ---- vertical-scoped memory (#680, Rule 7) -----------------

#[tokio::test]
async fn append_for_vertical_persists_vertical_id() {
    let (m, ws) = fresh().await;
    let entry = m
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            "alpha-row".into(),
            None,
            "alpha",
        )
        .await
        .unwrap();
    assert_eq!(entry.vertical_id.as_deref(), Some("alpha"));
}

#[tokio::test]
async fn list_for_vertical_filters_to_owned_rows_and_excludes_legacy() {
    let (m, ws) = fresh().await;
    // Three writes:
    //  - alpha-only entry (vertical-tagged)
    //  - beta-only entry (vertical-tagged)
    //  - legacy entry (workspace-global, no vertical_id)
    m.append_for_vertical(ws, MemoryFile::Memory, "alpha-row".into(), None, "alpha")
        .await
        .unwrap();
    m.append_for_vertical(ws, MemoryFile::Memory, "beta-row".into(), None, "beta")
        .await
        .unwrap();
    m.append(ws, MemoryFile::Memory, "legacy-row".into(), None)
        .await
        .unwrap();

    // alpha sees only its own entry — NOT beta's, NOT legacy.
    let alpha_view = m
        .list_for_vertical(ws, MemoryFile::Memory, "alpha")
        .await
        .unwrap();
    assert_eq!(alpha_view.len(), 1);
    assert_eq!(alpha_view[0].body, "alpha-row");
    assert_eq!(alpha_view[0].vertical_id.as_deref(), Some("alpha"));

    // beta sees only its own entry.
    let beta_view = m
        .list_for_vertical(ws, MemoryFile::Memory, "beta")
        .await
        .unwrap();
    assert_eq!(beta_view.len(), 1);
    assert_eq!(beta_view[0].body, "beta-row");

    // The legacy `list_active` still returns ALL three (back-
    // compat — gateway / CLI integration is a follow-up).
    let all = m.list_active(ws, MemoryFile::Memory).await.unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test]
async fn list_for_vertical_with_unknown_vertical_returns_empty() {
    let (m, ws) = fresh().await;
    m.append_for_vertical(ws, MemoryFile::Memory, "x".into(), None, "alpha")
        .await
        .unwrap();
    let ghost = m
        .list_for_vertical(ws, MemoryFile::Memory, "ghost")
        .await
        .unwrap();
    assert!(ghost.is_empty());
}

#[tokio::test]
async fn vertical_id_round_trips_through_get_and_list_active() {
    // The legacy `list_active` and `get` paths must surface
    // the new `vertical_id` field on rows that have it
    // (otherwise downstream serialization drops it).
    let (m, ws) = fresh().await;
    let written = m
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            "round-trip".into(),
            None,
            "alpha",
        )
        .await
        .unwrap();

    let fetched = m.get(written.id).await.unwrap();
    assert_eq!(fetched.vertical_id.as_deref(), Some("alpha"));

    let listed = m.list_active(ws, MemoryFile::Memory).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].vertical_id.as_deref(), Some("alpha"));
}
