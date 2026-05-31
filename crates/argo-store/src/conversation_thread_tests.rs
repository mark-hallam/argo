//! Unit tests for the `conversation_thread` store module.
//!
//! Covers the round-trip (insert → list), newest-first ordering, the
//! `limit` cap, tenant + worker isolation on the read (a caller can never
//! see another tenant's — or another worker's — threads), the
//! primary-key-collision invariant, the nullable `audit_href`, and the
//! `count_for_worker` idempotency probe.

use super::*;
use sqlx::sqlite::SqlitePoolOptions;

const T1: &str = "00000000-0000-0000-0000-000000000001";
const T2: &str = "00000000-0000-0000-0000-000000000002";
const W1: &[u8] = &[1u8; 16];
const W2: &[u8] = &[2u8; 16];

/// In-memory pool with the `worker_conversation_thread` table created via
/// the real versioned migration runner (v87) — exactly as the runtime
/// applies it.
async fn pool() -> SqlitePool {
    let p = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("memory pool");
    crate::migrations::run_up_to(&p, 87)
        .await
        .expect("migrations up to v87");
    p
}

fn ins<'a>(
    id: &'a str,
    tenant: &'a str,
    worker: &'a [u8],
    handle: &'a str,
    topic: &'a str,
    last_message_at: i64,
) -> ConversationThreadInsert<'a> {
    ConversationThreadInsert {
        id,
        tenant_id: tenant,
        worker_id: worker,
        counterparty_handle: handle,
        topic,
        last_message_at,
        audit_href: Some("/audit/action/x"),
    }
}

#[tokio::test]
async fn insert_then_list_round_trips() {
    let p = pool().await;
    insert(&p, &ins("ct_1", T1, W1, "Diego", "Interview", 100))
        .await
        .unwrap();
    let rows = list_for_worker(&p, T1, W1, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "ct_1");
    assert_eq!(rows[0].counterparty_handle, "Diego");
    assert_eq!(rows[0].topic, "Interview");
    assert_eq!(rows[0].last_message_at, 100);
    assert_eq!(rows[0].audit_href.as_deref(), Some("/audit/action/x"));
    assert_eq!(rows[0].worker_id, W1);
}

#[tokio::test]
async fn list_is_newest_first() {
    let p = pool().await;
    insert(&p, &ins("ct_old", T1, W1, "A", "old", 100))
        .await
        .unwrap();
    insert(&p, &ins("ct_new", T1, W1, "B", "new", 300))
        .await
        .unwrap();
    insert(&p, &ins("ct_mid", T1, W1, "C", "mid", 200))
        .await
        .unwrap();
    let rows = list_for_worker(&p, T1, W1, 10).await.unwrap();
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["ct_new", "ct_mid", "ct_old"]);
}

#[tokio::test]
async fn list_respects_limit() {
    let p = pool().await;
    for i in 0..5 {
        insert(
            &p,
            &ins(
                &format!("ct_{i}"),
                T1,
                W1,
                "handle",
                "topic",
                i64::from(i),
            ),
        )
        .await
        .unwrap();
    }
    let rows = list_for_worker(&p, T1, W1, 2).await.unwrap();
    assert_eq!(rows.len(), 2, "limit must cap the result set");
}

#[tokio::test]
async fn read_is_tenant_isolated() {
    let p = pool().await;
    insert(&p, &ins("ct_t1", T1, W1, "Diego", "Interview", 100))
        .await
        .unwrap();
    // A different tenant sees nothing — even with the same worker blob.
    let other = list_for_worker(&p, T2, W1, 10).await.unwrap();
    assert!(
        other.is_empty(),
        "tenant T2 must not see T1's thread, got {}",
        other.len()
    );
}

#[tokio::test]
async fn read_is_worker_isolated() {
    let p = pool().await;
    insert(&p, &ins("ct_w1", T1, W1, "Diego", "Interview", 100))
        .await
        .unwrap();
    // A different worker (same tenant) sees nothing.
    let other = list_for_worker(&p, T1, W2, 10).await.unwrap();
    assert!(
        other.is_empty(),
        "worker W2 must not see W1's thread, got {}",
        other.len()
    );
}

#[tokio::test]
async fn duplicate_id_is_invariant_error() {
    let p = pool().await;
    insert(&p, &ins("ct_dup", T1, W1, "A", "topic", 100))
        .await
        .unwrap();
    let err = insert(&p, &ins("ct_dup", T1, W1, "B", "topic", 200))
        .await
        .expect_err("duplicate id must fail");
    assert!(
        matches!(err, ArgoError::Invariant(_)),
        "expected Invariant on PK collision, got {err:?}"
    );
}

#[tokio::test]
async fn audit_href_can_be_null() {
    let p = pool().await;
    insert(
        &p,
        &ConversationThreadInsert {
            id: "ct_nohref",
            tenant_id: T1,
            worker_id: W1,
            counterparty_handle: "handle",
            topic: "topic",
            last_message_at: 100,
            audit_href: None,
        },
    )
    .await
    .unwrap();
    let rows = list_for_worker(&p, T1, W1, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].audit_href.is_none());
}

#[tokio::test]
async fn count_for_worker_is_tenant_and_worker_scoped() {
    let p = pool().await;
    assert_eq!(count_for_worker(&p, T1, W1).await.unwrap(), 0);
    insert(&p, &ins("ct_a", T1, W1, "A", "t", 1))
        .await
        .unwrap();
    insert(&p, &ins("ct_b", T1, W1, "B", "t", 2))
        .await
        .unwrap();
    // Noise under a different worker + tenant must not inflate the count.
    insert(&p, &ins("ct_c", T1, W2, "C", "t", 3))
        .await
        .unwrap();
    insert(&p, &ins("ct_d", T2, W1, "D", "t", 4))
        .await
        .unwrap();
    assert_eq!(count_for_worker(&p, T1, W1).await.unwrap(), 2);
}
