//! Tests for `grants.rs`. Sibling-mod pattern keeps the
//! production module under the 500-LOC budget.
#![cfg(test)]

use crate::SessionStore;

async fn setup_session_with_workspace(store: &SessionStore) -> argo_types::SessionId {
    let (tenant, workspace) = store.ensure_workspace("argo", "/tmp").await.unwrap();
    store
        .create_session(tenant, workspace, "mock", "mock", Some("t"), None)
        .await
        .unwrap()
}

#[tokio::test]
async fn add_then_list_round_trips_a_grant() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;

    let id = store
        .add_session_grant(sid, "delegate_task", "cli", Some("op approved"), false)
        .await
        .unwrap();

    let grants = store.list_session_grants(sid).await.unwrap();
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0].id, id);
    assert_eq!(grants[0].tool_name, "delegate_task");
    assert_eq!(grants[0].source, "cli");
    assert_eq!(grants[0].notes.as_deref(), Some("op approved"));
    assert!(!grants[0].is_one_shot);
    assert!(grants[0].consumed_at_secs.is_none());
}

#[tokio::test]
async fn list_returns_grants_newest_first() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;

    let _first = store
        .add_session_grant(sid, "a", "cli", None, false)
        .await
        .unwrap();
    // Sleep one second so the second grant has a higher
    // granted_at — sqlite's strftime('%s','now') is whole-second
    // granular and we order DESC by it.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let _second = store
        .add_session_grant(sid, "b", "cli", None, false)
        .await
        .unwrap();

    let grants = store.list_session_grants(sid).await.unwrap();
    assert_eq!(grants.len(), 2);
    assert_eq!(grants[0].tool_name, "b", "newest must be first");
    assert_eq!(grants[1].tool_name, "a");
}

#[tokio::test]
async fn list_returns_empty_for_session_with_no_grants() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;
    let grants = store.list_session_grants(sid).await.unwrap();
    assert!(grants.is_empty());
}

#[tokio::test]
async fn duplicate_grants_for_same_tool_are_allowed() {
    // The runtime de-duplicates by tool_name when building the
    // approval-set, so re-approving the same tool is idempotent
    // from the operator's POV — but the audit trail shows every
    // approval event. Lock that contract.
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;

    store
        .add_session_grant(sid, "delegate_task", "cli", None, false)
        .await
        .unwrap();
    store
        .add_session_grant(sid, "delegate_task", "gateway", Some("re-approved"), false)
        .await
        .unwrap();

    let grants = store.list_session_grants(sid).await.unwrap();
    assert_eq!(grants.len(), 2);
    assert_eq!(grants[0].source, "gateway");
    assert_eq!(grants[1].source, "cli");
}

#[tokio::test]
async fn revoke_removes_all_grants_for_tool_and_returns_count() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;

    store
        .add_session_grant(sid, "delegate_task", "cli", None, false)
        .await
        .unwrap();
    store
        .add_session_grant(sid, "delegate_task", "gateway", None, false)
        .await
        .unwrap();
    store
        .add_session_grant(sid, "other_tool", "cli", None, false)
        .await
        .unwrap();

    let removed = store
        .revoke_session_grants_for_tool(sid, "delegate_task")
        .await
        .unwrap();
    assert_eq!(removed, 2);

    let remaining = store.list_session_grants(sid).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].tool_name, "other_tool");
}

// ---- one-shot grants (#667) ----------------------------------

#[tokio::test]
async fn one_shot_grant_persists_with_is_one_shot_true() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;
    let _id = store
        .add_session_grant(sid, "terminal", "cli", None, true)
        .await
        .unwrap();
    let grants = store.list_session_grants(sid).await.unwrap();
    assert_eq!(grants.len(), 1);
    assert!(grants[0].is_one_shot);
    assert!(grants[0].consumed_at_secs.is_none());
}

#[tokio::test]
async fn consume_one_shot_grant_marks_oldest_unconsumed() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;

    let first = store
        .add_session_grant(sid, "terminal", "cli", None, true)
        .await
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let _second = store
        .add_session_grant(sid, "terminal", "cli", None, true)
        .await
        .unwrap();

    let consumed = store
        .consume_one_shot_grant(sid, "terminal")
        .await
        .unwrap();
    assert_eq!(consumed, Some(first), "oldest unconsumed must consume first");

    // Active list now shows only the second.
    let active = store.list_session_grants(sid).await.unwrap();
    assert_eq!(active.len(), 1);

    // include_consumed shows both.
    let all = store
        .list_session_grants_with_consumed(sid)
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
    let consumed_rows: Vec<&_> = all
        .iter()
        .filter(|g| g.consumed_at_secs.is_some())
        .collect();
    assert_eq!(consumed_rows.len(), 1);
    assert_eq!(consumed_rows[0].id, first);
}

#[tokio::test]
async fn consume_one_shot_grant_returns_none_when_no_active_grant() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;
    // Persistent grant exists but is NOT one-shot — consume
    // must skip it.
    store
        .add_session_grant(sid, "terminal", "cli", None, false)
        .await
        .unwrap();
    let consumed = store
        .consume_one_shot_grant(sid, "terminal")
        .await
        .unwrap();
    assert!(consumed.is_none());
    // Persistent grant is still active.
    let active = store.list_session_grants(sid).await.unwrap();
    assert_eq!(active.len(), 1);
    assert!(!active[0].is_one_shot);
}

#[tokio::test]
async fn list_active_excludes_consumed_one_shot_grants() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;
    store
        .add_session_grant(sid, "terminal", "cli", None, true)
        .await
        .unwrap();
    let _ = store
        .consume_one_shot_grant(sid, "terminal")
        .await
        .unwrap();
    let active = store.list_session_grants(sid).await.unwrap();
    assert!(active.is_empty(), "consumed one-shot grant must be hidden");
}
