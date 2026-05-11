//! Unit tests for [`super`] — per-subtask budget tree (Stream
//! `mission-delegation-budget-tree`). Covers the 8 cases listed in
//! the brief: set/spend round-trip, get_remaining shapes (declared,
//! NULL/legacy), walk_tree at 2- and 3-level depth, overspend
//! semantics, and the "set budget on a row that pre-existed without
//! one" path.
#![cfg(test)]

use super::*;
use crate::subtask::{insert, NewSubtask};
use crate::SessionStore;

async fn fresh_pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    store.pool().clone()
}

async fn seed_row(pool: &SqlitePool, path: &str, worker: &str) -> String {
    insert(
        pool,
        &NewSubtask {
            parent_goal: "delegate the launch",
            parent_session_id: Some("sess-budget"),
            worker_id: worker,
            plan_node_path: path,
            description: "d",
        },
    )
    .await
    .expect("insert")
}

#[tokio::test]
async fn set_budget_round_trip() {
    let pool = fresh_pool().await;
    let id = seed_row(&pool, "/root/a", "wkr-a").await;
    set_subtask_budget(&pool, &id, 1_000).await.expect("set");
    let remaining = get_remaining(&pool, &id).await.expect("get_remaining");
    assert_eq!(remaining, Some(1_000));
}

#[tokio::test]
async fn record_spend_accumulates_additively() {
    let pool = fresh_pool().await;
    let id = seed_row(&pool, "/root/b", "wkr-b").await;
    set_subtask_budget(&pool, &id, 500).await.unwrap();
    record_subtask_spend(&pool, &id, 100).await.unwrap();
    record_subtask_spend(&pool, &id, 250).await.unwrap();
    let remaining = get_remaining(&pool, &id).await.unwrap();
    assert_eq!(remaining, Some(500 - 350));
}

#[tokio::test]
async fn get_remaining_with_budget_returns_declared_minus_spent() {
    let pool = fresh_pool().await;
    let id = seed_row(&pool, "/root/c", "wkr-c").await;
    set_subtask_budget(&pool, &id, 200).await.unwrap();
    record_subtask_spend(&pool, &id, 75).await.unwrap();
    assert_eq!(get_remaining(&pool, &id).await.unwrap(), Some(125));
}

#[tokio::test]
async fn get_remaining_returns_none_for_unbounded_legacy_row() {
    let pool = fresh_pool().await;
    // Legacy / unbounded — row inserted without ever calling
    // set_subtask_budget. The brief requires this path to return
    // None (no enforcement) rather than crash.
    let id = seed_row(&pool, "/root/d", "wkr-d").await;
    record_subtask_spend(&pool, &id, 999).await.unwrap();
    assert_eq!(get_remaining(&pool, &id).await.unwrap(), None);
}

#[tokio::test]
async fn walk_tree_aggregates_two_levels() {
    let pool = fresh_pool().await;
    let root = seed_row(&pool, "/root/e", "wkr-root").await;
    let child_a = seed_row(&pool, "/root/e/a", "wkr-ca").await;
    let child_b = seed_row(&pool, "/root/e/b", "wkr-cb").await;
    set_subtask_parent(&pool, &child_a, &root).await.unwrap();
    set_subtask_parent(&pool, &child_b, &root).await.unwrap();
    set_subtask_budget(&pool, &root, 1_000).await.unwrap();
    record_subtask_spend(&pool, &root, 10).await.unwrap();
    record_subtask_spend(&pool, &child_a, 100).await.unwrap();
    record_subtask_spend(&pool, &child_b, 50).await.unwrap();
    let walked = walk_tree(&pool, &root).await.unwrap();
    assert_eq!(walked.len(), 3);
    let root_node = walked.iter().find(|w| w.row.id == root).unwrap();
    assert_eq!(root_node.aggregated_spent_usd_cents, 10 + 100 + 50);
    let a_node = walked.iter().find(|w| w.row.id == child_a).unwrap();
    assert_eq!(a_node.aggregated_spent_usd_cents, 100);
    let b_node = walked.iter().find(|w| w.row.id == child_b).unwrap();
    assert_eq!(b_node.aggregated_spent_usd_cents, 50);
}

#[tokio::test]
async fn walk_tree_aggregates_three_levels() {
    let pool = fresh_pool().await;
    let root = seed_row(&pool, "/root/f", "wkr-root").await;
    let mid = seed_row(&pool, "/root/f/mid", "wkr-mid").await;
    let leaf = seed_row(&pool, "/root/f/mid/leaf", "wkr-leaf").await;
    set_subtask_parent(&pool, &mid, &root).await.unwrap();
    set_subtask_parent(&pool, &leaf, &mid).await.unwrap();
    record_subtask_spend(&pool, &root, 1).await.unwrap();
    record_subtask_spend(&pool, &mid, 20).await.unwrap();
    record_subtask_spend(&pool, &leaf, 300).await.unwrap();
    let walked = walk_tree(&pool, &root).await.unwrap();
    let root_node = walked.iter().find(|w| w.row.id == root).unwrap();
    assert_eq!(root_node.aggregated_spent_usd_cents, 1 + 20 + 300);
    let mid_node = walked.iter().find(|w| w.row.id == mid).unwrap();
    assert_eq!(mid_node.aggregated_spent_usd_cents, 20 + 300);
    let leaf_node = walked.iter().find(|w| w.row.id == leaf).unwrap();
    assert_eq!(leaf_node.aggregated_spent_usd_cents, 300);
}

#[tokio::test]
async fn spend_exceeding_budget_still_records_and_remaining_goes_negative() {
    let pool = fresh_pool().await;
    let id = seed_row(&pool, "/root/g", "wkr-g").await;
    set_subtask_budget(&pool, &id, 100).await.unwrap();
    record_subtask_spend(&pool, &id, 150)
        .await
        .expect("overspend records cleanly");
    let remaining = get_remaining(&pool, &id).await.unwrap();
    assert_eq!(remaining, Some(-50));
}

#[tokio::test]
async fn set_budget_on_row_inserted_before_budget_columns_existed() {
    // Models the "legacy row" promotion path: a subtask written by
    // the original `record_subtask` flow predates ever having a
    // budget set. We can still pin one on later via set_subtask_budget,
    // and get_remaining flips from None → Some(b).
    let pool = fresh_pool().await;
    let id = seed_row(&pool, "/root/h", "wkr-h").await;
    assert_eq!(get_remaining(&pool, &id).await.unwrap(), None);
    set_subtask_budget(&pool, &id, 400).await.unwrap();
    record_subtask_spend(&pool, &id, 25).await.unwrap();
    assert_eq!(get_remaining(&pool, &id).await.unwrap(), Some(375));
}
