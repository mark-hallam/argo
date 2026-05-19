//! Unit tests for the `trust_streak::streaks_for_worker` aggregator
//! (Stream `mission-pm-trust-ratchet-ui`).
//!
//! Three fixture scenarios per the brief's acceptance list:
//! 1. clean streak — N consecutive grants, no resets;
//! 2. broken-by-denial — `approval.replied` with `reply_kind=reject`
//!    mid-stream resets the count;
//! 3. broken-by-failure — `approval.revoked` mid-stream resets the
//!    count (the in-tree stand-in for `worker_action.failed`).

use super::{streaks_for_worker, TrustStreak};
use crate::SessionStore;
use uuid::Uuid;

const WORKER_A: &str = "00000000-0000-0000-0000-000000000001";

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn insert_event(
    store: &SessionStore,
    seq_id: u128,
    created_at: i64,
    kind: &str,
    worker_id: &str,
    tool: &str,
    payload: serde_json::Value,
) {
    sqlx::query(
        "INSERT INTO audit_record \
         (id, session_id, kind, payload_json, parent_hash, self_hash, created_at, worker_id, action_type) \
         VALUES (?, NULL, ?, ?, x'00', x'01', ?, ?, ?)",
    )
    .bind(Uuid::from_u128(seq_id).as_bytes().as_slice())
    .bind(kind)
    .bind(payload.to_string())
    .bind(created_at)
    .bind(worker_id)
    .bind(tool)
    .execute(store.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn clean_streak_counts_consecutive_grants() {
    let store = fresh_store().await;
    for i in 0..12 {
        insert_event(
            &store,
            i + 1,
            (i as i64) * 10,
            "approval.granted",
            WORKER_A,
            "update_roadmap",
            serde_json::json!({"tool": "update_roadmap"}),
        )
        .await;
    }

    let got: Vec<TrustStreak> = streaks_for_worker(&store, WORKER_A, 10).await.unwrap();
    assert_eq!(got.len(), 1, "one (tool, target_glob) pair");
    assert_eq!(got[0].tool, "update_roadmap");
    assert_eq!(got[0].streak_count, 12);
    assert_eq!(got[0].last_decided_at_micros, 110);
    assert!(got[0].target_glob.is_none());
}

#[tokio::test]
async fn denial_resets_streak() {
    let store = fresh_store().await;
    // 5 grants
    for i in 0..5 {
        insert_event(
            &store,
            i + 1,
            (i as i64) * 10,
            "approval.granted",
            WORKER_A,
            "update_roadmap",
            serde_json::json!({"tool": "update_roadmap"}),
        )
        .await;
    }
    // operator replies "reject" — mid-stream denial
    insert_event(
        &store,
        100,
        60,
        "approval.replied",
        WORKER_A,
        "update_roadmap",
        serde_json::json!({"reply_kind": "reject", "tool": "update_roadmap"}),
    )
    .await;
    // 3 more grants AFTER the denial; the streak restarts from 1
    for i in 0..3 {
        insert_event(
            &store,
            200 + i,
            70 + (i as i64) * 10,
            "approval.granted",
            WORKER_A,
            "update_roadmap",
            serde_json::json!({"tool": "update_roadmap"}),
        )
        .await;
    }

    // With min=4, the post-reset streak of 3 is filtered out; the
    // pre-reset run of 5 already ended, so the answer is empty.
    let got_high = streaks_for_worker(&store, WORKER_A, 4).await.unwrap();
    assert!(
        got_high.is_empty(),
        "denial must reset; nothing reaches threshold: {got_high:?}",
    );

    // With min=3, the post-reset run of 3 surfaces.
    let got_low = streaks_for_worker(&store, WORKER_A, 3).await.unwrap();
    assert_eq!(got_low.len(), 1);
    assert_eq!(got_low[0].streak_count, 3);
}

#[tokio::test]
async fn revocation_resets_streak() {
    let store = fresh_store().await;
    for i in 0..6 {
        insert_event(
            &store,
            i + 1,
            (i as i64) * 10,
            "approval.granted",
            WORKER_A,
            "post_status",
            serde_json::json!({"tool": "post_status"}),
        )
        .await;
    }
    // revocation — operator changed their mind on a prior grant.
    insert_event(
        &store,
        500,
        70,
        "approval.revoked",
        WORKER_A,
        "post_status",
        serde_json::json!({"tool": "post_status"}),
    )
    .await;
    // 2 more grants after revocation; new streak starts at 1.
    for i in 0..2 {
        insert_event(
            &store,
            600 + i,
            80 + (i as i64) * 10,
            "approval.granted",
            WORKER_A,
            "post_status",
            serde_json::json!({"tool": "post_status"}),
        )
        .await;
    }

    let got = streaks_for_worker(&store, WORKER_A, 5).await.unwrap();
    assert!(
        got.is_empty(),
        "revocation must reset; pre-revoke streak ended at the revoke row",
    );
    let got_low = streaks_for_worker(&store, WORKER_A, 2).await.unwrap();
    assert_eq!(got_low.len(), 1);
    assert_eq!(got_low[0].streak_count, 2);
}

#[tokio::test]
async fn target_glob_partitions_streaks() {
    let store = fresh_store().await;
    // Two streaks under the same tool but different target_glob.
    for i in 0..4 {
        insert_event(
            &store,
            i + 1,
            (i as i64) * 10,
            "approval.granted",
            WORKER_A,
            "update_roadmap",
            serde_json::json!({"tool": "update_roadmap", "target_glob": "Q3-*"}),
        )
        .await;
    }
    for i in 0..2 {
        insert_event(
            &store,
            200 + i,
            100 + (i as i64) * 10,
            "approval.granted",
            WORKER_A,
            "update_roadmap",
            serde_json::json!({"tool": "update_roadmap", "target_glob": "Q4-*"}),
        )
        .await;
    }

    let got = streaks_for_worker(&store, WORKER_A, 2).await.unwrap();
    assert_eq!(got.len(), 2, "one row per (tool, target_glob) pair");
    assert_eq!(got[0].streak_count, 4, "longest first");
    assert_eq!(got[0].target_glob.as_deref(), Some("Q3-*"));
    assert_eq!(got[1].streak_count, 2);
    assert_eq!(got[1].target_glob.as_deref(), Some("Q4-*"));
}

#[tokio::test]
async fn min_threshold_filters_out_short_streaks() {
    let store = fresh_store().await;
    for i in 0..3 {
        insert_event(
            &store,
            i + 1,
            (i as i64) * 10,
            "approval.granted",
            WORKER_A,
            "noop",
            serde_json::json!({"tool": "noop"}),
        )
        .await;
    }
    let got = streaks_for_worker(&store, WORKER_A, 10).await.unwrap();
    assert!(got.is_empty(), "streak of 3 < min 10");
    let got_low = streaks_for_worker(&store, WORKER_A, 3).await.unwrap();
    assert_eq!(got_low.len(), 1);
}
