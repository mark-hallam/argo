//! Unit tests for [`super`] — the persona_revision append-only log
//! and revert flow.
#![cfg(test)]

use super::*;
use crate::SessionStore;

async fn seed_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn seed_persona_row(store: &SessionStore, worker_id: Uuid, name: &str, voice: &str) {
    let row_id = Uuid::now_v7().to_string();
    let tenant_id = Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO persona \
         (id, worker_id, tenant_id, name, voice_preamble, avatar, bio, \
          builder_attribution, provenance_chain_json) \
         VALUES (?, ?, ?, ?, ?, NULL, NULL, ?, '[]')",
    )
    .bind(row_id)
    .bind(worker_id.to_string())
    .bind(tenant_id)
    .bind(name)
    .bind(voice)
    .bind("operator@example.test")
    .execute(store.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn record_appends_row() {
    let store = seed_store().await;
    let worker_id = Uuid::now_v7();

    let rev = record_persona_change(
        store.pool(),
        worker_id,
        "voice",
        "old voice",
        "new voice",
        "cli:bob@example.test",
    )
    .await
    .unwrap();

    assert_eq!(rev.field, "voice");
    assert_eq!(rev.old_value, "old voice");
    assert_eq!(rev.new_value, "new voice");
    assert_eq!(rev.actor, "cli:bob@example.test");
    assert_eq!(rev.audit_chain_hash.len(), 64);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM persona_revision WHERE worker_id = ?")
        .bind(worker_id.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn list_returns_newest_first() {
    let store = seed_store().await;
    let worker_id = Uuid::now_v7();

    // Three sequential revisions. record_persona_change stamps
    // Utc::now() at second resolution, so within one test tick all
    // three rows can land at the same second; force a strict order
    // with explicit revised_at writes so the newest-first assertion
    // doesn't depend on the wall clock.
    let first = record_persona_change(store.pool(), worker_id, "voice", "a", "b", "sys")
        .await
        .unwrap();
    let mid = record_persona_change(store.pool(), worker_id, "voice", "b", "c", "sys")
        .await
        .unwrap();
    let last = record_persona_change(store.pool(), worker_id, "voice", "c", "d", "sys")
        .await
        .unwrap();

    for (rev_id, ts) in [(first.rev_id, 50_i64), (mid.rev_id, 100), (last.rev_id, 200)] {
        sqlx::query("UPDATE persona_revision SET revised_at = ? WHERE rev_id = ?")
            .bind(ts)
            .bind(rev_id.to_string())
            .execute(store.pool())
            .await
            .unwrap();
    }

    let revisions = list_persona_revisions(store.pool(), worker_id, 50)
        .await
        .unwrap();
    assert_eq!(revisions.len(), 3);
    assert_eq!(revisions[0].rev_id, last.rev_id);
    assert_eq!(revisions[1].rev_id, mid.rev_id);
}

#[tokio::test]
async fn revert_applies_old_value() {
    let store = seed_store().await;
    let worker_id = Uuid::now_v7();
    seed_persona_row(&store, worker_id, "Maya", "initial voice").await;

    let rev = record_persona_change(
        store.pool(),
        worker_id,
        "voice",
        "initial voice",
        "updated voice",
        "cli:alice",
    )
    .await
    .unwrap();
    sqlx::query("UPDATE persona SET voice_preamble = ? WHERE worker_id = ?")
        .bind("updated voice")
        .bind(worker_id.to_string())
        .execute(store.pool())
        .await
        .unwrap();

    revert_persona_to(store.pool(), rev.rev_id, "cli:bob")
        .await
        .unwrap();

    let voice: String =
        sqlx::query_scalar("SELECT voice_preamble FROM persona WHERE worker_id = ?")
            .bind(worker_id.to_string())
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(voice, "initial voice");
}

#[tokio::test]
async fn revert_records_itself() {
    let store = seed_store().await;
    let worker_id = Uuid::now_v7();
    seed_persona_row(&store, worker_id, "Maya", "v1").await;
    let rev = record_persona_change(store.pool(), worker_id, "voice", "v1", "v2", "sys")
        .await
        .unwrap();
    sqlx::query("UPDATE persona SET voice_preamble = 'v2' WHERE worker_id = ?")
        .bind(worker_id.to_string())
        .execute(store.pool())
        .await
        .unwrap();

    let revert_rev = revert_persona_to(store.pool(), rev.rev_id, "cli:carol")
        .await
        .unwrap();

    assert_eq!(revert_rev.field, "voice");
    assert_eq!(revert_rev.old_value, "v2");
    assert_eq!(revert_rev.new_value, "v1");
    assert_eq!(revert_rev.actor, "cli:carol");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM persona_revision WHERE worker_id = ?")
        .bind(worker_id.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn audit_chain_prev_link() {
    let store = seed_store().await;
    let worker_id = Uuid::now_v7();

    let r1 = record_persona_change(store.pool(), worker_id, "voice", "", "v1", "sys")
        .await
        .unwrap();
    let r2 = record_persona_change(store.pool(), worker_id, "voice", "v1", "v2", "sys")
        .await
        .unwrap();

    assert_ne!(r1.audit_chain_hash, r2.audit_chain_hash);
    let prev = decode_hex_32(&r1.audit_chain_hash).unwrap();
    let expected = hex_encode(&compute_revision_hash(&prev, &r2.rev_id, "voice", "v2"));
    assert_eq!(r2.audit_chain_hash, expected);
    verify_chain(store.pool(), worker_id).await.unwrap();
}

#[tokio::test]
async fn tamper_detected() {
    let store = seed_store().await;
    let worker_id = Uuid::now_v7();
    let r1 = record_persona_change(store.pool(), worker_id, "voice", "", "v1", "sys")
        .await
        .unwrap();
    record_persona_change(store.pool(), worker_id, "voice", "v1", "v2", "sys")
        .await
        .unwrap();

    // Flip a byte in r1's stored new_value — the chain on the next
    // row no longer matches the recomputed hash.
    sqlx::query("UPDATE persona_revision SET new_value = 'tampered' WHERE rev_id = ?")
        .bind(r1.rev_id.to_string())
        .execute(store.pool())
        .await
        .unwrap();

    let err = verify_chain(store.pool(), worker_id)
        .await
        .expect_err("chain must report tamper");
    assert!(err.to_string().contains("tampered"), "err: {err}");
}

#[tokio::test]
async fn cross_worker_isolation() {
    let store = seed_store().await;
    let worker_a = Uuid::now_v7();
    let worker_b = Uuid::now_v7();
    record_persona_change(store.pool(), worker_a, "voice", "", "a1", "sys")
        .await
        .unwrap();
    record_persona_change(store.pool(), worker_b, "voice", "", "b1", "sys")
        .await
        .unwrap();
    record_persona_change(store.pool(), worker_a, "voice", "a1", "a2", "sys")
        .await
        .unwrap();

    let a = list_persona_revisions(store.pool(), worker_a, 50)
        .await
        .unwrap();
    let b = list_persona_revisions(store.pool(), worker_b, 50)
        .await
        .unwrap();
    assert_eq!(a.len(), 2);
    assert_eq!(b.len(), 1);
    for r in &a {
        assert_eq!(r.worker_id, worker_a);
    }
    assert_eq!(b[0].worker_id, worker_b);
}

#[tokio::test]
async fn concurrent_writes_no_collision() {
    let store = seed_store().await;
    let worker_id = Uuid::now_v7();

    let mut handles = Vec::new();
    for i in 0..8 {
        let pool = store.pool().clone();
        handles.push(tokio::spawn(async move {
            record_persona_change(
                &pool,
                worker_id,
                "voice",
                &format!("v{i}"),
                &format!("v{}", i + 1),
                "sys",
            )
            .await
            .unwrap()
        }));
    }
    for h in handles {
        h.await.unwrap();
    }

    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM persona_revision WHERE worker_id = ?")
        .bind(worker_id.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(rows, 8);

    let distinct_hashes: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT audit_chain_hash) FROM persona_revision WHERE worker_id = ?",
    )
    .bind(worker_id.to_string())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(distinct_hashes, 8, "every concurrent write gets a unique hash");
}
