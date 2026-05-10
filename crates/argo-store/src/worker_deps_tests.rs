//! Pool-level tests for `worker_deps`. Cycle detection, cross-kind
//! coexistence, and the impact (in-edge BFS) closure are the
//! invariants that justify a separate module — the brief calls out
//! 12 unit tests for these paths.

use super::*;
use crate::SessionStore;
use sqlx::SqlitePool;

async fn pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    ensure_schema(store.pool()).await.expect("apply deps schema");
    store.pool().clone()
}

#[tokio::test]
async fn add_then_list_round_trip() {
    let pool = pool().await;
    let inserted = add_dependency(&pool, "A", "B", DepKind::Data, Some("ledger pulls"))
        .await
        .expect("insert");
    assert!(inserted);

    let out = list_for_worker(&pool, "A", Direction::Out).await.unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].a_worker_id, "A");
    assert_eq!(out[0].b_worker_id, "B");
    assert_eq!(out[0].kind, DepKind::Data);
    assert_eq!(out[0].note.as_deref(), Some("ledger pulls"));
    assert!(out[0].declared_at > 0);

    let inbound = list_for_worker(&pool, "B", Direction::In).await.unwrap();
    assert_eq!(inbound.len(), 1);
    assert_eq!(inbound[0].a_worker_id, "A");
}

#[tokio::test]
async fn cycle_rejected_same_kind() {
    let pool = pool().await;
    add_dependency(&pool, "A", "B", DepKind::Data, None)
        .await
        .unwrap();
    let err = add_dependency(&pool, "B", "A", DepKind::Data, None)
        .await
        .expect_err("should reject");
    let msg = err.to_string();
    assert!(msg.contains("cycle detected"), "msg: {msg}");
    assert!(msg.contains("B"), "msg: {msg}");
    assert!(msg.contains("A"), "msg: {msg}");
}

#[tokio::test]
async fn longer_cycle_rejected_same_kind() {
    let pool = pool().await;
    add_dependency(&pool, "A", "B", DepKind::Handoff, None)
        .await
        .unwrap();
    add_dependency(&pool, "B", "C", DepKind::Handoff, None)
        .await
        .unwrap();
    let err = add_dependency(&pool, "C", "A", DepKind::Handoff, None)
        .await
        .expect_err("should reject 3-cycle");
    let msg = err.to_string();
    assert!(msg.contains("cycle"), "msg: {msg}");
    // Path should mention every hop in the cycle: C, A, B, C
    for n in ["A", "B", "C"] {
        assert!(msg.contains(n), "msg missing {n}: {msg}");
    }
}

#[tokio::test]
async fn cross_kind_cycle_allowed() {
    let pool = pool().await;
    add_dependency(&pool, "A", "B", DepKind::Data, None)
        .await
        .unwrap();
    // Reverse edge in a different kind is fine.
    add_dependency(&pool, "B", "A", DepKind::Approval, None)
        .await
        .expect("cross-kind allowed");

    let both = list_for_worker(&pool, "A", Direction::Both).await.unwrap();
    assert_eq!(both.len(), 2);
    let kinds: Vec<DepKind> = both.iter().map(|r| r.kind).collect();
    assert!(kinds.contains(&DepKind::Data));
    assert!(kinds.contains(&DepKind::Approval));
}

#[tokio::test]
async fn self_dependency_rejected() {
    let pool = pool().await;
    let err = add_dependency(&pool, "A", "A", DepKind::Data, None)
        .await
        .expect_err("self-edge rejected");
    assert!(err.to_string().contains("self-dependency"));
}

#[tokio::test]
async fn duplicate_add_updates_note_and_declared_at() {
    let pool = pool().await;
    add_dependency(&pool, "A", "B", DepKind::Data, Some("first"))
        .await
        .unwrap();
    let original = list_for_worker(&pool, "A", Direction::Out).await.unwrap()[0].declared_at;
    // sleep enough that timestamp_micros advances
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    add_dependency(&pool, "A", "B", DepKind::Data, Some("second"))
        .await
        .unwrap();
    let rows = list_for_worker(&pool, "A", Direction::Out).await.unwrap();
    assert_eq!(rows.len(), 1, "PK collapses duplicate");
    assert_eq!(rows[0].note.as_deref(), Some("second"));
    assert!(
        rows[0].declared_at >= original,
        "declared_at refreshed: {} vs {}",
        rows[0].declared_at,
        original
    );
}

#[tokio::test]
async fn remove_returns_rows_affected() {
    let pool = pool().await;
    add_dependency(&pool, "A", "B", DepKind::Data, None)
        .await
        .unwrap();
    assert_eq!(
        remove_dependency(&pool, "A", "B", DepKind::Data)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        remove_dependency(&pool, "A", "B", DepKind::Data)
            .await
            .unwrap(),
        0
    );
    assert!(list_for_worker(&pool, "A", Direction::Out)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn impact_empty_graph() {
    let pool = pool().await;
    let imp = impact(&pool, "A").await.unwrap();
    assert!(imp.is_empty());
}

#[tokio::test]
async fn impact_returns_transitive_dependents_sorted_by_depth() {
    let pool = pool().await;
    // Chain: D → C → B → A. So if A breaks, B/C/D are impacted.
    add_dependency(&pool, "B", "A", DepKind::Data, None).await.unwrap();
    add_dependency(&pool, "C", "B", DepKind::Data, None).await.unwrap();
    add_dependency(&pool, "D", "C", DepKind::Data, None).await.unwrap();
    // E has no relation, must NOT appear in the impact set.
    add_dependency(&pool, "E", "F", DepKind::Data, None).await.unwrap();
    let imp = impact(&pool, "A").await.unwrap();
    let pairs: Vec<(u32, &str)> = imp
        .iter()
        .map(|i| (i.depth, i.worker_id.as_str()))
        .collect();
    assert_eq!(pairs, vec![(1, "B"), (2, "C"), (3, "D")]);
}

#[tokio::test]
async fn impact_walks_across_kinds() {
    let pool = pool().await;
    // Mixed-kind chain: B depends on A (data), C depends on B (approval).
    add_dependency(&pool, "B", "A", DepKind::Data, None).await.unwrap();
    add_dependency(&pool, "C", "B", DepKind::Approval, None).await.unwrap();
    let imp = impact(&pool, "A").await.unwrap();
    let ids: Vec<&str> = imp.iter().map(|i| i.worker_id.as_str()).collect();
    assert_eq!(ids, vec!["B", "C"]);
}

#[tokio::test]
async fn impact_depth_is_capped() {
    let pool = pool().await;
    // Build a chain longer than IMPACT_MAX_DEPTH so the walk truncates.
    let chain_len = (IMPACT_MAX_DEPTH + 3) as usize;
    for i in 0..chain_len {
        let dependent = format!("w{}", i + 1);
        let dependency = format!("w{}", i);
        add_dependency(&pool, &dependent, &dependency, DepKind::Data, None)
            .await
            .unwrap();
    }
    let imp = impact(&pool, "w0").await.unwrap();
    let max_depth = imp.iter().map(|i| i.depth).max().unwrap();
    assert!(
        max_depth <= IMPACT_MAX_DEPTH,
        "depth {max_depth} exceeds cap {}",
        IMPACT_MAX_DEPTH,
    );
    assert_eq!(imp.len() as u32, IMPACT_MAX_DEPTH);
}

#[tokio::test]
async fn list_all_dumps_workspace_graph_sorted() {
    let pool = pool().await;
    add_dependency(&pool, "B", "A", DepKind::Data, None).await.unwrap();
    add_dependency(&pool, "A", "C", DepKind::Approval, None).await.unwrap();
    add_dependency(&pool, "C", "B", DepKind::Skill, None).await.unwrap();
    let all = list_all(&pool).await.unwrap();
    assert_eq!(all.len(), 3);
    // Sort ordering: kind ASC then a_worker_id ASC then b_worker_id ASC.
    // Approval < Data < Handoff < Skill on string comparison.
    assert_eq!(all[0].kind, DepKind::Approval);
    assert_eq!(all[0].a_worker_id, "A");
    assert_eq!(all[1].kind, DepKind::Data);
    assert_eq!(all[2].kind, DepKind::Skill);
}

#[test]
fn dep_kind_parse_round_trip() {
    for kind in DepKind::ALL {
        assert_eq!(DepKind::parse(kind.as_str()).unwrap(), kind);
    }
    assert!(DepKind::parse("nope").is_err());
}

#[test]
fn direction_parse_recognised_tokens() {
    assert!(matches!(Direction::parse("out").unwrap(), Direction::Out));
    assert!(matches!(Direction::parse("in").unwrap(), Direction::In));
    assert!(matches!(Direction::parse("both").unwrap(), Direction::Both));
    assert!(Direction::parse("sideways").is_err());
}
