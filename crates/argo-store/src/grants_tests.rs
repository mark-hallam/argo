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
        .add_session_grant(sid, "delegate_task", "cli", Some("op approved"), false, None)
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
    assert!(grants[0].target_glob.is_none());
}

#[tokio::test]
async fn list_returns_grants_newest_first() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;

    let _first = store
        .add_session_grant(sid, "a", "cli", None, false, None)
        .await
        .unwrap();
    // Sleep one second so the second grant has a higher
    // granted_at — sqlite's strftime('%s','now') is whole-second
    // granular and we order DESC by it.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let _second = store
        .add_session_grant(sid, "b", "cli", None, false, None)
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
        .add_session_grant(sid, "delegate_task", "cli", None, false, None)
        .await
        .unwrap();
    store
        .add_session_grant(
            sid,
            "delegate_task",
            "gateway",
            Some("re-approved"),
            false,
            None,
        )
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
        .add_session_grant(sid, "delegate_task", "cli", None, false, None)
        .await
        .unwrap();
    store
        .add_session_grant(sid, "delegate_task", "gateway", None, false, None)
        .await
        .unwrap();
    store
        .add_session_grant(sid, "other_tool", "cli", None, false, None)
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
        .add_session_grant(sid, "terminal", "cli", None, true, None)
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
        .add_session_grant(sid, "terminal", "cli", None, true, None)
        .await
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let _second = store
        .add_session_grant(sid, "terminal", "cli", None, true, None)
        .await
        .unwrap();

    let consumed = store.consume_one_shot_grant(sid, "terminal").await.unwrap();
    assert_eq!(
        consumed,
        Some(first),
        "oldest unconsumed must consume first"
    );

    // Active list now shows only the second.
    let active = store.list_session_grants(sid).await.unwrap();
    assert_eq!(active.len(), 1);

    // include_consumed shows both.
    let all = store.list_session_grants_with_consumed(sid).await.unwrap();
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
        .add_session_grant(sid, "terminal", "cli", None, false, None)
        .await
        .unwrap();
    let consumed = store.consume_one_shot_grant(sid, "terminal").await.unwrap();
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
        .add_session_grant(sid, "terminal", "cli", None, true, None)
        .await
        .unwrap();
    let _ = store.consume_one_shot_grant(sid, "terminal").await.unwrap();
    let active = store.list_session_grants(sid).await.unwrap();
    assert!(active.is_empty(), "consumed one-shot grant must be hidden");
}

// ---- per-target glob (mission-pm-approval-target-scope) -------

#[tokio::test]
async fn find_grant_with_null_target_glob_matches_any_target() {
    // Case (a) — back-compat invariant. A session-global grant
    // (target_glob = NULL) authorises a dispatch with any
    // target value, including None.
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;
    let id = store
        .add_session_grant(sid, "update_roadmap", "cli", None, false, None)
        .await
        .unwrap();

    let hit = store
        .find_session_grant_for_dispatch(sid, "update_roadmap", Some("Q3-2026-L42"))
        .await
        .unwrap()
        .expect("session-global grant must match any target");
    assert_eq!(hit.id, id);
    assert!(hit.target_glob.is_none());

    let hit_none = store
        .find_session_grant_for_dispatch(sid, "update_roadmap", None)
        .await
        .unwrap()
        .expect("session-global grant must match a None target");
    assert_eq!(hit_none.id, id);
}

#[tokio::test]
async fn find_grant_with_glob_matches_only_satisfying_targets() {
    // Case (b) — glob "Q3-*" matches "Q3-2026-L42" and refuses
    // "Q4-2026-L01". With no fallback the refusal surfaces as
    // ArgoError::Provider("approval grant target mismatch").
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;
    let id = store
        .add_session_grant(sid, "update_roadmap", "cli", None, false, Some("Q3-*"))
        .await
        .unwrap();

    let hit = store
        .find_session_grant_for_dispatch(sid, "update_roadmap", Some("Q3-2026-L42"))
        .await
        .unwrap()
        .expect("glob 'Q3-*' must cover 'Q3-2026-L42'");
    assert_eq!(hit.id, id);
    assert_eq!(hit.target_glob.as_deref(), Some("Q3-*"));

    let err = store
        .find_session_grant_for_dispatch(sid, "update_roadmap", Some("Q4-2026-L01"))
        .await
        .expect_err("glob 'Q3-*' must reject 'Q4-2026-L01'");
    assert!(
        err.to_string().contains("approval grant target mismatch"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn find_grant_with_target_glob_requires_target_on_consume() {
    // Case (c) — a target-bound grant cannot apply when the
    // dispatch supplies no target.
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;
    let _id = store
        .add_session_grant(sid, "update_roadmap", "cli", None, false, Some("Q3-*"))
        .await
        .unwrap();

    let err = store
        .find_session_grant_for_dispatch(sid, "update_roadmap", None)
        .await
        .expect_err("target-bound grant must refuse a None target");
    assert!(
        err.to_string().contains("approval grant target mismatch"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn one_shot_grant_with_target_consumes_on_first_match() {
    // Case (d) — a one-shot grant with target_glob consumes on
    // the first matching dispatch; the next matching dispatch
    // surfaces "target mismatch" because the only grant is gone.
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;
    let id = store
        .add_session_grant(sid, "update_roadmap", "cli", None, true, Some("Q3-*"))
        .await
        .unwrap();

    // First dispatch finds the grant.
    let hit = store
        .find_session_grant_for_dispatch(sid, "update_roadmap", Some("Q3-2026-L42"))
        .await
        .unwrap()
        .expect("first dispatch must match");
    assert_eq!(hit.id, id);
    assert!(hit.is_one_shot);

    // Consumer marks the grant consumed (the runner does this
    // post-dispatch via `consume_one_shot_grant`).
    let consumed = store
        .consume_one_shot_grant(sid, "update_roadmap")
        .await
        .unwrap();
    assert_eq!(consumed, Some(id));

    // Second dispatch sees no active grant for the tool at all —
    // Ok(None), the gate stays in effect.
    let after = store
        .find_session_grant_for_dispatch(sid, "update_roadmap", Some("Q3-2026-L42"))
        .await
        .unwrap();
    assert!(
        after.is_none(),
        "consumed one-shot grant must not re-match"
    );
}

#[tokio::test]
async fn two_grants_on_same_tool_apply_most_specific_match() {
    // Case (e) — a session-global grant and a target-bound grant
    // coexist on the same tool. When the target satisfies the
    // narrower glob, the narrower grant wins. When it does not,
    // the session-global grant is the back-compat fallback.
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;

    let global_id = store
        .add_session_grant(sid, "update_roadmap", "cli", None, false, None)
        .await
        .unwrap();
    let narrow_id = store
        .add_session_grant(sid, "update_roadmap", "cli", None, false, Some("Q3-*"))
        .await
        .unwrap();

    let narrow_hit = store
        .find_session_grant_for_dispatch(sid, "update_roadmap", Some("Q3-2026-L42"))
        .await
        .unwrap()
        .expect("narrower grant must apply to a matching target");
    assert_eq!(
        narrow_hit.id, narrow_id,
        "most-specific (target-bound) grant must win over session-global"
    );

    let fallback_hit = store
        .find_session_grant_for_dispatch(sid, "update_roadmap", Some("Q4-2026-L01"))
        .await
        .unwrap()
        .expect("session-global fallback must apply when narrow grant misses");
    assert_eq!(
        fallback_hit.id, global_id,
        "non-matching target falls back to the session-global grant"
    );
}

#[tokio::test]
async fn migration_is_idempotent() {
    // Case (f) — running `migrate()` repeatedly is a no-op once
    // v75 has applied; existing rows retain their v8 columns and
    // gain a NULL target_glob (back-compat invariant).
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let sid = setup_session_with_workspace(&store).await;
    let id = store
        .add_session_grant(sid, "delegate_task", "cli", Some("pre-rerun"), false, None)
        .await
        .unwrap();

    // Second migrate call must not error and must not destroy
    // the row inserted between calls.
    store.migrate().await.unwrap();
    store.migrate().await.unwrap();

    let grants = store.list_session_grants(sid).await.unwrap();
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0].id, id);
    assert!(grants[0].target_glob.is_none());
    assert_eq!(grants[0].notes.as_deref(), Some("pre-rerun"));
}
