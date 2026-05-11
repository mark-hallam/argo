//! Tests for `worker_archive::delete_worker` /
//! `force_delete_worker` (Stream `mission-identity-delete-worker`).
//!
//! Lives in a sibling file rather than `worker_archive_tests.rs` so the
//! existing 326-LOC archive/rehydrate test module stays under the
//! 500-LOC ceiling once the delete suite lands.

use super::*;
use crate::SessionStore;
use sqlx::SqlitePool;
use uuid::Uuid;

const IDENTITY_SCHEMA_SQL: &str =
    include_str!("../migrations/12__worker_identity.sql");

/// Mirrors the helper in `worker_archive_tests::pool` — re-applies the
/// `argo-identity` schema atop a fully-migrated store so seed helpers
/// can insert into `worker` / `persona` / `role` / `duty` without
/// pulling in the `argo-identity` cycle.
async fn pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    for stmt in IDENTITY_SCHEMA_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .expect("apply identity schema");
    }
    for (name, ty) in [
        ("worker_id", "TEXT"),
        ("charter", "TEXT"),
        ("kpis_json", "TEXT"),
        ("escalation", "TEXT"),
    ] {
        let _ = sqlx::query(&format!("ALTER TABLE role ADD COLUMN {name} {ty}"))
            .execute(store.pool())
            .await;
    }
    ensure_schema(store.pool()).await.expect("apply archive schema");
    ensure_audit_tombstone_column(store.pool())
        .await
        .expect("apply audit tombstone column");
    store.pool().clone()
}

async fn seed_live_worker(pool: &SqlitePool, worker_id: &str, tenant_id: &str) {
    sqlx::query(
        "INSERT INTO persona
            (id, worker_id, tenant_id, name, voice_preamble, avatar, bio,
             builder_attribution, provenance_chain_json)
         VALUES (?, ?, ?, 'Maya', 'op@example.test', NULL, NULL, 'op@example.test', '[]')",
    )
    .bind(format!("persona-{worker_id}"))
    .bind(worker_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("seed persona");

    sqlx::query(
        "INSERT INTO role
            (tenant_id, name, inherits_from, capabilities, worker_id,
             charter, kpis_json, escalation)
         VALUES (?, ?, NULL, '[]', ?, NULL, NULL, NULL)",
    )
    .bind(tenant_id.as_bytes())
    .bind(format!("worker:{worker_id}"))
    .bind(worker_id)
    .execute(pool)
    .await
    .expect("seed role");

    sqlx::query(
        "INSERT INTO worker
            (id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
             auto_approval_json, channels_json, created_at)
         VALUES (?, ?, 'support', ?, ?, 'worker_self', '{}', '[]', 0)",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .bind(format!("persona-{worker_id}"))
    .bind(format!("worker:{worker_id}"))
    .execute(pool)
    .await
    .expect("seed worker");
}

async fn archive_then_age(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
    archived_at_micros: i64,
) {
    seed_live_worker(pool, worker_id, tenant_id).await;
    archive_worker(pool, worker_id, tenant_id, None, None, "{}")
        .await
        .expect("archive ok");
    sqlx::query("UPDATE worker_archive SET archived_at = ? WHERE id = ?")
        .bind(archived_at_micros)
        .bind(worker_id)
        .execute(pool)
        .await
        .expect("backdate archived_at");
}

async fn workspace_id_for_seed(pool: &SqlitePool, tenant_id: &str) -> Vec<u8> {
    let workspace_id = Uuid::now_v7();
    let tenant_uuid = Uuid::parse_str(tenant_id).unwrap();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, ?, '/tmp/test', 0)",
    )
    .bind(workspace_id.as_bytes().as_slice())
    .bind(tenant_uuid.as_bytes().as_slice())
    .bind(format!("ws-{workspace_id}"))
    .execute(pool)
    .await
    .expect("seed workspace");
    workspace_id.as_bytes().to_vec()
}

async fn seed_memory_entry(
    pool: &SqlitePool,
    workspace_blob: &[u8],
    worker_id: &str,
    tenant_id: &str,
) -> Vec<u8> {
    let entry_id = Uuid::now_v7();
    let worker_blob = Uuid::parse_str(worker_id).unwrap().as_bytes().to_vec();
    let tenant_blob = Uuid::parse_str(tenant_id).unwrap().as_bytes().to_vec();
    sqlx::query(
        "INSERT INTO memory_entry
            (id, workspace_id, file, body, learned_in, learned_at, hash,
             tenant_id, worker_id, memory_scope, layer)
         VALUES (?, ?, 'WORKER.md', 'note', NULL, 0, ?, ?, ?, 'worker_self', 'recent')",
    )
    .bind(entry_id.as_bytes().as_slice())
    .bind(workspace_blob)
    .bind(b"hash".to_vec())
    .bind(&tenant_blob)
    .bind(&worker_blob)
    .execute(pool)
    .await
    .expect("seed memory_entry");
    entry_id.as_bytes().to_vec()
}

async fn seed_session_and_audit(
    pool: &SqlitePool,
    workspace_blob: &[u8],
    worker_id: &str,
    tenant_id: &str,
) -> Vec<u8> {
    let session_id = Uuid::now_v7();
    let audit_id = Uuid::now_v7();
    let worker_blob = Uuid::parse_str(worker_id).unwrap().as_bytes().to_vec();
    let tenant_blob = Uuid::parse_str(tenant_id).unwrap().as_bytes().to_vec();
    sqlx::query(
        "INSERT INTO session
            (id, tenant_id, workspace_id, started_at, model, provider, worker_id)
         VALUES (?, ?, ?, 0, 'm', 'p', ?)",
    )
    .bind(session_id.as_bytes().as_slice())
    .bind(&tenant_blob)
    .bind(workspace_blob)
    .bind(&worker_blob)
    .execute(pool)
    .await
    .expect("seed session");
    sqlx::query(
        "INSERT INTO audit_record
            (id, session_id, kind, payload_json, parent_hash, self_hash, created_at)
         VALUES (?, ?, 'test', '{}', ?, ?, 0)",
    )
    .bind(audit_id.as_bytes().as_slice())
    .bind(session_id.as_bytes().as_slice())
    .bind(b"parent".to_vec())
    .bind(b"self".to_vec())
    .execute(pool)
    .await
    .expect("seed audit_record");
    audit_id.as_bytes().to_vec()
}

#[tokio::test]
async fn delete_rejects_when_worker_is_still_live() {
    let pool = pool().await;
    let worker_id = "11111111-1111-1111-1111-111111111111";
    let tenant_id = "22222222-2222-2222-2222-222222222222";
    seed_live_worker(&pool, worker_id, tenant_id).await;

    let err = delete_worker(&pool, worker_id, tenant_id)
        .await
        .expect_err("live worker rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("still live") && msg.contains("archive it first"),
        "unexpected error: {msg}"
    );
}

#[tokio::test]
async fn delete_rejects_inside_grace_window_with_remaining_days() {
    let pool = pool().await;
    let worker_id = "33333333-3333-3333-3333-333333333333";
    let tenant_id = "44444444-4444-4444-4444-444444444444";
    // Archived 1 day ago in a 30-day window → 29-ish days remaining.
    let one_day_ago = Utc::now().timestamp_micros() - MICROS_PER_DAY;
    archive_then_age(&pool, worker_id, tenant_id, one_day_ago).await;

    let err = delete_worker(&pool, worker_id, tenant_id)
        .await
        .expect_err("inside grace window");
    let msg = err.to_string();
    assert!(msg.contains("grace window"), "unexpected error: {msg}");
    assert!(
        msg.contains("day(s) remaining"),
        "remaining-days line missing: {msg}"
    );

    // The archive row is still present — error path must not delete.
    let still_archived =
        list_archived(&pool, tenant_id).await.expect("list archived");
    assert_eq!(still_archived.len(), 1);
}

#[tokio::test]
async fn delete_succeeds_after_grace_and_drops_archive_row() {
    let pool = pool().await;
    let worker_id = "55555555-5555-5555-5555-555555555555";
    let tenant_id = "66666666-6666-6666-6666-666666666666";
    let well_past = Utc::now().timestamp_micros() - 60 * MICROS_PER_DAY;
    archive_then_age(&pool, worker_id, tenant_id, well_past).await;

    delete_worker(&pool, worker_id, tenant_id)
        .await
        .expect("delete after grace");

    let archive_after =
        list_archived(&pool, tenant_id).await.expect("list");
    assert!(
        archive_after.is_empty(),
        "archive row dropped on successful delete"
    );
}

#[tokio::test]
async fn delete_cascades_memory_rows_within_tenant_only() {
    let pool = pool().await;
    let worker_id = "77777777-7777-7777-7777-777777777777";
    let tenant_id = "88888888-8888-8888-8888-888888888888";
    let other_tenant = "99999999-9999-9999-9999-999999999999";
    let well_past = Utc::now().timestamp_micros() - 60 * MICROS_PER_DAY;
    archive_then_age(&pool, worker_id, tenant_id, well_past).await;

    let workspace_a = workspace_id_for_seed(&pool, tenant_id).await;
    let workspace_b = workspace_id_for_seed(&pool, other_tenant).await;
    seed_memory_entry(&pool, &workspace_a, worker_id, tenant_id).await;
    let cross_id =
        seed_memory_entry(&pool, &workspace_b, worker_id, other_tenant).await;

    delete_worker(&pool, worker_id, tenant_id)
        .await
        .expect("delete cascade");

    let in_tenant: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM memory_entry WHERE worker_id = ? AND tenant_id = ?",
    )
    .bind(Uuid::parse_str(worker_id).unwrap().as_bytes().as_slice())
    .bind(Uuid::parse_str(tenant_id).unwrap().as_bytes().as_slice())
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(in_tenant.is_none(), "in-tenant memory row purged");

    let cross_tenant: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT id FROM memory_entry WHERE id = ?")
            .bind(cross_id.as_slice())
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert!(
        cross_tenant.is_some(),
        "cross-tenant memory row preserved (per-tenant cascade)"
    );
}

#[tokio::test]
async fn delete_tombstones_audit_rows_without_breaking_chain_hashes() {
    let pool = pool().await;
    let worker_id = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let tenant_id = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
    let well_past = Utc::now().timestamp_micros() - 60 * MICROS_PER_DAY;
    archive_then_age(&pool, worker_id, tenant_id, well_past).await;

    let workspace = workspace_id_for_seed(&pool, tenant_id).await;
    let audit_id =
        seed_session_and_audit(&pool, &workspace, worker_id, tenant_id).await;

    delete_worker(&pool, worker_id, tenant_id)
        .await
        .expect("delete with audit tombstone");

    let row = sqlx::query(
        "SELECT worker_deleted_at, parent_hash, self_hash FROM audit_record WHERE id = ?",
    )
    .bind(audit_id.as_slice())
    .fetch_one(&pool)
    .await
    .expect("audit row preserved");
    let stamped: Option<i64> = row.get("worker_deleted_at");
    let parent_hash: Vec<u8> = row.get("parent_hash");
    let self_hash: Vec<u8> = row.get("self_hash");
    assert!(stamped.is_some(), "tombstone stamped");
    assert_eq!(parent_hash, b"parent", "chain parent_hash preserved");
    assert_eq!(self_hash, b"self", "chain self_hash preserved");
}

#[tokio::test]
async fn force_delete_skips_grace_gate_but_demands_confirm() {
    let pool = pool().await;
    let worker_id = "cccccccc-cccc-cccc-cccc-cccccccccccc";
    let tenant_id = "dddddddd-dddd-dddd-dddd-dddddddddddd";
    // Archived just now → well inside any grace window.
    let just_now = Utc::now().timestamp_micros();
    archive_then_age(&pool, worker_id, tenant_id, just_now).await;

    let refused = force_delete_worker(&pool, worker_id, tenant_id, false)
        .await
        .expect_err("confirm=false rejected");
    assert!(refused.to_string().contains("confirm flag"));

    force_delete_worker(&pool, worker_id, tenant_id, true)
        .await
        .expect("force delete with confirm");

    let archive_after =
        list_archived(&pool, tenant_id).await.expect("list");
    assert!(archive_after.is_empty(), "archive row purged by force path");
}
