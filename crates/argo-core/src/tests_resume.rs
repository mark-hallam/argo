//! Resume-on-restart tests (Amendment 13): orphan-tool-call scan,
//! restart-policy decision matrix, and discard semantics.
//!
//! Loaded as a child of `tests` via `#[path = "tests_resume.rs"] mod`.
//! Helpers are local to this module.

use super::*;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use std::sync::Arc;

async fn fresh_store_with_session() -> (Arc<SessionStore>, SessionId) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
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
    let session = store
        .create_session(tenant, ws, "model", "provider", Some("t"), None)
        .await
        .unwrap();
    (store, session)
}

async fn insert_orphan_tool_call(
    store: &SessionStore,
    session: SessionId,
    name: &str,
    started_at_micros: i64,
) {
    let id = uuid::Uuid::now_v7();
    let msg_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO message (id, session_id, role, content_json, created_at, seq)
         VALUES (?, ?, 'assistant', '[]', ?, 1)",
    )
    .bind(msg_id.as_bytes().as_slice())
    .bind(session.0.as_bytes().as_slice())
    .bind(started_at_micros)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tool_call (id, session_id, message_id, name, arguments_json, started_at)
         VALUES (?, ?, ?, ?, '{}', ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(session.0.as_bytes().as_slice())
    .bind(msg_id.as_bytes().as_slice())
    .bind(name)
    .bind(started_at_micros)
    .execute(store.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn scan_returns_only_orphan_tool_calls() {
    let (store, session) = fresh_store_with_session().await;
    let now = chrono::Utc::now();
    let started = (now - chrono::Duration::seconds(30)).timestamp_micros();
    insert_orphan_tool_call(&store, session, "file_write", started).await;
    let scanned = scan_interrupted(&store, now).await.unwrap();
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].session_id, session);
    assert_eq!(scanned[0].last_tool_name, "file_write");
    assert!(scanned[0].age_seconds >= 29 && scanned[0].age_seconds <= 31);
}

#[tokio::test]
async fn apply_restart_policy_prompt_user_wraps_each_session() {
    let (_, session) = fresh_store_with_session().await;
    let interrupted = vec![InterruptedSession {
        session_id: session,
        last_tool_name: "x".into(),
        last_tool_arguments_json: "{}".into(),
        started_at: chrono::Utc::now(),
        age_seconds: 5,
    }];
    let acts = apply_restart_policy(RestartPolicy::PromptUser, interrupted);
    assert_eq!(acts.len(), 1);
    matches!(acts[0], RestartAction::Prompt { .. });
}

#[tokio::test]
async fn apply_restart_policy_auto_resume_attaches_prefix() {
    let (_, session) = fresh_store_with_session().await;
    let interrupted = vec![InterruptedSession {
        session_id: session,
        last_tool_name: "file_read".into(),
        last_tool_arguments_json: "{}".into(),
        started_at: chrono::Utc::now(),
        age_seconds: 5,
    }];
    let acts = apply_restart_policy(RestartPolicy::AutoResume, interrupted);
    if let RestartAction::Resume { prefix, .. } = &acts[0] {
        assert!(prefix.contains("interrupted"));
    } else {
        panic!("expected Resume");
    }
}

#[tokio::test]
async fn discard_marks_session_ended_with_restart_reason() {
    let (store, session) = fresh_store_with_session().await;
    let now = chrono::Utc::now();
    insert_orphan_tool_call(
        &store,
        session,
        "file_write",
        now.timestamp_micros() - 30_000_000,
    )
    .await;
    mark_session_discarded(&store, session, now).await.unwrap();
    let row: (Option<i64>, Option<String>) =
        sqlx::query_as("SELECT ended_at, end_reason FROM session WHERE id = ?")
            .bind(session.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert!(row.0.is_some());
    assert_eq!(row.1.as_deref(), Some("restart_discard"));
}
