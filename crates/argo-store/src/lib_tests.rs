//! Tests for [`super::SessionStore`] — extracted from `lib.rs` to
//! keep the production-code file under the 500-LOC ceiling.
#![cfg(test)]
use super::*;
use argo_types::MessageRole;

#[tokio::test]
async fn open_and_migrate() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 42);
}

#[tokio::test]
async fn migrate_round_trip() {
    let store = SessionStore::open(":memory:").await.unwrap();
    crate::migrations::run_up_to(store.pool(), 11).await.unwrap();
    store.migrate_down_to(0).await.unwrap();
    assert_eq!(store.schema_version().await.unwrap(), 0);

    store.migrate().await.unwrap();
    let err = store
        .migrate_down_to(11)
        .await
        .expect_err("v12+ down unsupported below v16");
    assert!(err.to_string().contains("rollback unsupported"));
    // v17..v21 unwind cleanly; v16 refuses (orchestrator session
    // columns may own live data), so the floor settles at 16.
    assert_eq!(store.schema_version().await.unwrap(), 16);
}

#[tokio::test]
async fn ensure_workspace_is_idempotent_by_root_path() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let (t1, w1) = store.ensure_workspace("smoke", "/tmp/x").await.unwrap();
    let (t2, w2) = store.ensure_workspace("smoke", "/tmp/x").await.unwrap();
    assert_eq!(t1, t2);
    assert_eq!(w1, w2);
    let (_t3, w3) = store.ensure_workspace("smoke", "/tmp/y").await.unwrap();
    assert_ne!(w1, w3);
}

async fn fresh_store_with_workspace() -> (SessionStore, TenantId, WorkspaceId) {
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
    (store, tenant, ws)
}

#[tokio::test]
async fn spend_ledger_records_and_aggregates_session_total() {
    let (store, tenant, ws) = fresh_store_with_workspace().await;
    let session = store
        .create_session(tenant, ws, "m", "p", None, None)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    store
        .record_spend(
            tenant,
            session,
            now.timestamp_micros(),
            250_000,
            "anth",
            "haiku",
        )
        .await
        .unwrap();
    store
        .record_spend(
            tenant,
            session,
            now.timestamp_micros(),
            750_000,
            "anth",
            "haiku",
        )
        .await
        .unwrap();
    let counters = store
        .load_spend_counters(tenant, session, now)
        .await
        .unwrap();
    assert_eq!(counters.session_used, 1_000_000);
    assert_eq!(counters.day_used, 1_000_000);
    assert_eq!(counters.month_used, 1_000_000);
}

#[tokio::test]
async fn spend_ledger_isolates_by_tenant() {
    let (store, tenant_a, ws_a) = fresh_store_with_workspace().await;
    let session_a = store
        .create_session(tenant_a, ws_a, "m", "p", None, None)
        .await
        .unwrap();
    let tenant_b = TenantId::new();
    let ws_b = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'b', '.', strftime('%s','now'))",
    )
    .bind(ws_b.0.as_bytes().as_slice())
    .bind(tenant_b.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let session_b = store
        .create_session(tenant_b, ws_b, "m", "p", None, None)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    store
        .record_spend(
            tenant_b,
            session_b,
            now.timestamp_micros(),
            5_000_000,
            "p",
            "m",
        )
        .await
        .unwrap();
    let a = store
        .load_spend_counters(tenant_a, session_a, now)
        .await
        .unwrap();
    assert_eq!(a.session_used, 0);
    assert_eq!(a.day_used, 0);
    assert_eq!(a.month_used, 0);
}

#[tokio::test]
async fn spend_ledger_day_window_excludes_yesterday() {
    let (store, tenant, ws) = fresh_store_with_workspace().await;
    let session = store
        .create_session(tenant, ws, "m", "p", None, None)
        .await
        .unwrap();
    let now = chrono::Utc::now();
    let yesterday = now - chrono::Duration::days(2);
    store
        .record_spend(tenant, session, yesterday.timestamp_micros(), 123, "p", "m")
        .await
        .unwrap();
    let counters = store
        .load_spend_counters(tenant, session, now)
        .await
        .unwrap();
    assert_eq!(counters.session_used, 123);
    assert_eq!(counters.day_used, 0);
}

#[tokio::test]
async fn create_session_and_append_messages() {
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
    let session = store
        .create_session(tenant, ws, "claude-haiku", "anthropic", Some("test"), None)
        .await
        .unwrap();
    store
        .append_message(
            session,
            MessageRole::User,
            r#"[{"type":"text","text":"hi"}]"#,
        )
        .await
        .unwrap();
    store
        .append_message(
            session,
            MessageRole::Assistant,
            r#"[{"type":"text","text":"hello"}]"#,
        )
        .await
        .unwrap();
    assert_eq!(store.message_count(session).await.unwrap(), 2);
}

#[tokio::test]
async fn list_sessions_recency_order() {
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
    for i in 0..3 {
        store
            .create_session(tenant, ws, "model", "provider", Some(&format!("s{i}")), None)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    let list = store.list_sessions(tenant, 10).await.unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].title.as_deref(), Some("s2"));
}

#[tokio::test]
async fn fts5_table_exists() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='message_fts'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn override_judge_verdict_updates_in_place() {
    use argo_types::JudgeRecordId;
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let (tenant, ws) = store.ensure_workspace("ws", "/tmp/jo").await.unwrap();
    let sid = store
        .create_session(tenant, ws, "m", "p", Some("t"), None)
        .await
        .unwrap();
    // Insert a judge_record row directly so we don't have to spin
    // up the full judge engine just to test override semantics.
    let record_id = JudgeRecordId::new();
    sqlx::query(
        "INSERT INTO judge_record (id, session_id, subject_kind, subject_ref, verdict, \
         reason, judge_provider, judge_model, rubric_id, rubric_version, \
         evidence_hashes, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(record_id.0.as_bytes().as_slice())
    .bind(sid.0.as_bytes().as_slice())
    .bind("turn")
    .bind("turn-1")
    .bind("partial")
    .bind(Option::<&str>::Some("orig reason"))
    .bind("anthropic")
    .bind("claude-x")
    .bind("default")
    .bind("v1")
    .bind("[]")
    .bind(0_i64)
    .execute(store.pool())
    .await
    .unwrap();

    let outcome = store
        .override_judge_verdict(record_id, "success", Some("explicit override"))
        .await
        .unwrap();
    assert_eq!(outcome, Some((sid, "partial".to_string())));

    let verdict: String =
        sqlx::query_scalar("SELECT verdict FROM judge_record WHERE id = ?")
            .bind(record_id.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(verdict, "success");
}

#[tokio::test]
async fn start_then_finish_forge_run_round_trips() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    let id = store
        .start_forge_run("my-skill", "0.1", 5)
        .await
        .unwrap();
    let row = store.get_forge_run(id).await.unwrap().unwrap();
    assert_eq!(row.status, "running");
    assert!(row.ended_at.is_none());
    assert_eq!(row.iterations, 0);
    assert!(row.final_score.is_none());

    let affected = store
        .finish_forge_run(id, "succeeded", Some(0.842), 5)
        .await
        .unwrap();
    assert_eq!(affected, 1);
    let row = store.get_forge_run(id).await.unwrap().unwrap();
    assert_eq!(row.status, "succeeded");
    assert!(row.ended_at.is_some());
    assert_eq!(row.iterations, 5);
    assert_eq!(row.final_score, Some(0.842));
}

#[tokio::test]
async fn record_marketplace_artifact_appends_and_lists_newest_first() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let (_tenant, workspace) = store.ensure_workspace("ws", "/tmp/m").await.unwrap();

    store
        .record_marketplace_artifact(
            workspace,
            "ghcr.io/argo/x:0.1",
            "sha256:aaa",
            "cli:test",
        )
        .await
        .unwrap();
    // Bump the clock so the second row sorts strictly after.
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    store
        .record_marketplace_artifact(
            workspace,
            "ghcr.io/argo/x:0.2",
            "sha256:bbb",
            "cli:test",
        )
        .await
        .unwrap();

    let rows = store.list_marketplace_artifacts(workspace, 10).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].oci_ref, "ghcr.io/argo/x:0.2");
    assert_eq!(rows[1].oci_ref, "ghcr.io/argo/x:0.1");
}

#[tokio::test]
async fn abort_forge_run_marks_status_and_ended_at() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO forge_run (id, skill_name, started_at, ended_at, base_version, \
         candidate_version, examples_count, iterations, spend_usd_micros, final_score, status) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind("my-skill")
    .bind(0_i64)
    .bind(Option::<i64>::None)
    .bind("0.1")
    .bind(Option::<&str>::None)
    .bind(0_i64)
    .bind(0_i64)
    .bind(0_i64)
    .bind(Option::<f64>::None)
    .bind("running")
    .execute(store.pool())
    .await
    .unwrap();

    let affected = store.abort_forge_run(id).await.unwrap();
    assert_eq!(affected, 1);

    let row = store.get_forge_run(id).await.unwrap().unwrap();
    assert_eq!(row.status, "aborted");
    assert!(row.ended_at.is_some());

    // Re-aborting an unknown id is a no-op.
    let none = store
        .abort_forge_run(uuid::Uuid::now_v7())
        .await
        .unwrap();
    assert_eq!(none, 0);
}

#[tokio::test]
async fn update_forge_run_progress_only_touches_running_rows() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    // Insert one running + one aborted forge_run row.
    let running = uuid::Uuid::now_v7();
    let aborted = uuid::Uuid::now_v7();
    for (id, status) in [(running, "running"), (aborted, "aborted")] {
        sqlx::query(
            "INSERT INTO forge_run (id, skill_name, started_at, ended_at, base_version, \
             candidate_version, examples_count, iterations, spend_usd_micros, final_score, status) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind("k")
        .bind(0_i64)
        .bind(Option::<i64>::None)
        .bind("0.1")
        .bind(Option::<&str>::None)
        .bind(1_i64)
        .bind(0_i64)
        .bind(0_i64)
        .bind(Option::<f64>::None)
        .bind(status)
        .execute(store.pool())
        .await
        .unwrap();
    }

    let affected = store
        .update_forge_run_progress(running, 3, Some(0.7), Some("0.4"))
        .await
        .unwrap();
    assert_eq!(affected, 1);
    let row = store.get_forge_run(running).await.unwrap().unwrap();
    assert_eq!(row.iterations, 3);
    assert_eq!(row.final_score, Some(0.7));
    assert_eq!(row.candidate_version.as_deref(), Some("0.4"));
    assert_eq!(row.status, "running");

    // The aborted row's WHERE clause filter rejects the update.
    let zero = store
        .update_forge_run_progress(aborted, 5, Some(0.9), Some("0.6"))
        .await
        .unwrap();
    assert_eq!(zero, 0);
    let row = store.get_forge_run(aborted).await.unwrap().unwrap();
    assert_eq!(row.iterations, 0);
    assert_eq!(row.final_score, None);
    assert_eq!(row.status, "aborted");
}

#[tokio::test]
async fn override_judge_verdict_returns_none_for_unknown_id() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let result = store
        .override_judge_verdict(
            argo_types::JudgeRecordId::new(),
            "success",
            None,
        )
        .await
        .unwrap();
    assert!(result.is_none());
}
