//! Unit tests for [`crate::retro`].
//!
//! Drives the store against an in-memory SQLite. The aggregator test
//! seeds `audit_record` directly so we don't pull in `argo-audit`'s
//! chain-hashing machinery just to count rows.

use super::*;
use crate::SessionStore;
use sqlx::Sqlite;
use uuid::Uuid;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    store
}

fn worker_blob(n: u8) -> Vec<u8> {
    let mut v = vec![0u8; 16];
    v[15] = n;
    v
}

fn tenant_blob(n: u8) -> Vec<u8> {
    let mut v = vec![0u8; 16];
    v[14] = n;
    v
}

fn new_retro(tenant: u8, worker: u8, now: i64) -> NewRetro {
    NewRetro {
        tenant_id: tenant_blob(tenant),
        worker_id: worker_blob(worker),
        window_start: now - 7 * 24 * 3600,
        window_end: now,
        metrics_json: metrics_template(),
        created_at: now,
    }
}

#[tokio::test]
async fn create_then_get_roundtrip() {
    let store = fresh_store().await;
    let s = RetroStore::new(store.pool());
    let id = s
        .create(new_retro(1, 1, 1_700_000_000))
        .await
        .expect("create");

    let row = s.get(id).await.expect("get");
    assert_eq!(row.id, id);
    assert_eq!(row.tenant_id, tenant_blob(1));
    assert_eq!(row.worker_id, worker_blob(1));
    assert_eq!(row.window_end, 1_700_000_000);
    assert!(!row.is_finalized());
    // Fresh retro starts with the 5-section empty template.
    assert_eq!(row.sections_json["surface_metrics"], "");
    assert_eq!(row.sections_json["what_went_well"], "");
    assert_eq!(row.sections_json["action_items"], "");
}

#[tokio::test]
async fn get_unknown_id_returns_not_found() {
    let store = fresh_store().await;
    let s = RetroStore::new(store.pool());
    let err = s.get(Uuid::nil()).await.unwrap_err();
    assert!(matches!(err, RetroError::NotFound), "got {err:?}");
}

#[tokio::test]
async fn list_for_worker_orders_newest_first() {
    let store = fresh_store().await;
    let s = RetroStore::new(store.pool());
    let older = s
        .create(new_retro(1, 1, 1_700_000_000))
        .await
        .expect("create older");
    let newer = s
        .create(new_retro(1, 1, 1_700_001_000))
        .await
        .expect("create newer");
    // Different worker — must NOT appear in worker=1's list.
    let _other = s
        .create(new_retro(1, 2, 1_700_000_500))
        .await
        .expect("other");

    let rows = s
        .list_for_worker(&worker_blob(1), 10)
        .await
        .expect("list");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, newer, "newest first");
    assert_eq!(rows[1].id, older);
}

#[tokio::test]
async fn update_sections_persists_then_blocks_on_finalize() {
    let store = fresh_store().await;
    let s = RetroStore::new(store.pool());
    let id = s
        .create(new_retro(1, 1, 1_700_000_000))
        .await
        .expect("create");

    let draft = serde_json::json!({
        "surface_metrics": "12 actions, 9 approved",
        "what_went_well": "Cold-start latency stayed under target.",
        "surprises": "",
        "what_to_change": "",
        "action_items": "",
    });
    s.update_sections(id, &draft).await.expect("update draft");

    let row = s.get(id).await.expect("get after update");
    assert_eq!(row.sections_json["what_went_well"], "Cold-start latency stayed under target.");

    s.finalize(id, 1_700_000_500).await.expect("finalize");
    let locked = s.get(id).await.expect("get finalized");
    assert!(locked.is_finalized());
    assert_eq!(locked.finalized_at, Some(1_700_000_500));

    // Subsequent edits return Finalized.
    let err = s.update_sections(id, &draft).await.unwrap_err();
    assert!(matches!(err, RetroError::Finalized), "got {err:?}");
    // Double-finalize → Finalized too.
    let err = s.finalize(id, 1_700_000_999).await.unwrap_err();
    assert!(matches!(err, RetroError::Finalized), "got {err:?}");
}

#[tokio::test]
async fn list_all_theme_search_matches_sections() {
    let store = fresh_store().await;
    let s = RetroStore::new(store.pool());
    let m = s
        .create(new_retro(1, 1, 1_700_000_000))
        .await
        .expect("matching");
    let _other = s
        .create(new_retro(1, 2, 1_700_000_500))
        .await
        .expect("other");

    let draft_match = serde_json::json!({
        "surface_metrics": "",
        "what_went_well": "Great escalation discipline this week.",
        "surprises": "",
        "what_to_change": "",
        "action_items": "",
    });
    let draft_other = serde_json::json!({
        "surface_metrics": "",
        "what_went_well": "Smooth handoffs.",
        "surprises": "",
        "what_to_change": "",
        "action_items": "",
    });
    s.update_sections(m, &draft_match).await.expect("update match");
    s.update_sections(_other, &draft_other)
        .await
        .expect("update other");

    let rows = s
        .list_all(&tenant_blob(1), Some("escalation"), 10)
        .await
        .expect("search");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, m);

    // Cross-tenant isolation: tenant=2 sees nothing.
    let rows = s
        .list_all(&tenant_blob(2), None, 10)
        .await
        .expect("cross tenant");
    assert!(rows.is_empty());
}

#[tokio::test]
async fn aggregate_metrics_counts_kinds_for_worker() {
    let store = fresh_store().await;
    let pool = store.pool();
    let worker = worker_blob(7);
    let session = seed_session_with_worker(pool, &worker).await;

    // Seed audit rows: 3 tool_call, 2 approval.granted, 1 approval.revoked,
    // 1 escalation.required, 1 skill.approved, 1 halt, 1 rbac.deny.
    let now = 1_700_000_000_i64;
    insert_audit(pool, &session, "tool_call", now).await;
    insert_audit(pool, &session, "tool_call", now + 10).await;
    insert_audit(pool, &session, "tool_call", now + 20).await;
    insert_audit(pool, &session, "approval.granted", now + 30).await;
    insert_audit(pool, &session, "approval.granted", now + 40).await;
    insert_audit(pool, &session, "approval.revoked", now + 50).await;
    insert_audit(pool, &session, "escalation.required", now + 60).await;
    insert_audit(pool, &session, "skill.approved", now + 70).await;
    insert_audit(pool, &session, "halt", now + 80).await;
    insert_audit(pool, &session, "rbac.deny", now + 90).await;

    let metrics = aggregate_metrics(pool, &worker, now - 1, now + 1_000)
        .await
        .expect("aggregate");
    assert_eq!(metrics["actions_total"], 3);
    assert_eq!(metrics["actions_approved"], 2);
    assert_eq!(metrics["actions_refused"], 1);
    assert_eq!(metrics["escalations"], 1);
    assert_eq!(metrics["learned_skills_added"], 1);
    assert_eq!(metrics["error_count_by_category"]["halt"], 1);
    assert_eq!(metrics["error_count_by_category"]["rbac.deny"], 1);
}

async fn seed_session_with_worker(pool: &sqlx::Pool<Sqlite>, worker_id: &[u8]) -> Vec<u8> {
    // The session table requires a workspace row first.
    let tenant = tenant_blob(9);
    let ws_id = vec![1u8; 16];
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, 'ws-test', '/tmp/ws-test', 1)",
    )
    .bind(&ws_id)
    .bind(&tenant)
    .execute(pool)
    .await
    .expect("workspace insert");
    let sid = vec![2u8; 16];
    sqlx::query(
        "INSERT INTO session (id, tenant_id, workspace_id, started_at, model, provider, \
         token_in_total, token_out_total, spend_usd_micros, worker_id) \
         VALUES (?, ?, ?, 1, 'm', 'p', 0, 0, 0, ?)",
    )
    .bind(&sid)
    .bind(&tenant)
    .bind(&ws_id)
    .bind(worker_id)
    .execute(pool)
    .await
    .expect("session insert");
    sid
}

async fn insert_audit(pool: &sqlx::Pool<Sqlite>, session_id: &[u8], kind: &str, t: i64) {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at) \
         VALUES (?, ?, ?, '{}', ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(session_id)
    .bind(kind)
    .bind(vec![0u8; 32])
    .bind(vec![1u8; 32])
    .bind(t)
    .execute(pool)
    .await
    .expect("audit insert");
}
