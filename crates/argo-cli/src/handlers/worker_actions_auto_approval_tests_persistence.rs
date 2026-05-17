//! Persisted-spend integration tests for the auto-approval gate.

use super::*;
use argo_identity::worker_budget;

#[tokio::test]
async fn persisted_spend_blocks_second_action_over_daily_cap() {
    let (store, memory, ws) = open_test_memory().await;
    let policy = enabled_policy(1_000_000);
    let first = rec_for_worker("act-cap-1", "worker-a");
    seed(&memory, ws, &first).await;
    let first_decl = cost_decl(Some(0.70));
    let now = at(6, 12);

    let effective = effective_from(policy.clone());
    let first_decision = maybe_auto_approve_with_pool(
        &memory,
        ws,
        &first.action_id,
        &first,
        Some(&first_decl),
        &effective,
        &empty_ctx(),
        store.pool(),
        now,
    )
    .await
    .unwrap();
    assert!(first_decision.is_auto_approved());
    assert_eq!(
        worker_budget::spend_today_for_worker(store.pool(), "worker-a", now)
            .await
            .unwrap(),
        700_000
    );

    let second = rec_for_worker("act-cap-2", "worker-a");
    seed(&memory, ws, &second).await;
    let second_decl = cost_decl(Some(0.40));
    let err = maybe_auto_approve_with_pool(
        &memory,
        ws,
        &second.action_id,
        &second,
        Some(&second_decl),
        &effective,
        &empty_ctx(),
        store.pool(),
        now,
    )
    .await
    .expect_err("second action should cross daily cap");
    assert!(err.to_string().contains("daily_cap"));
}

#[tokio::test]
async fn exactly_at_cap_still_allows_zero_cost_action() {
    let (store, memory, ws) = open_test_memory().await;
    let policy = enabled_policy(1_000_000);
    let effective = effective_from(policy.clone());
    let now = at(6, 12);
    let paid = rec_for_worker("act-exact-1", "worker-a");
    seed(&memory, ws, &paid).await;
    let paid_decl = cost_decl(Some(1.0));
    maybe_auto_approve_with_pool(
        &memory,
        ws,
        &paid.action_id,
        &paid,
        Some(&paid_decl),
        &effective,
        &empty_ctx(),
        store.pool(),
        now,
    )
    .await
    .unwrap();

    let free = rec_for_worker("act-exact-2", "worker-a");
    seed(&memory, ws, &free).await;
    let free_decl = cost_decl(None);
    let decision = maybe_auto_approve_with_pool(
        &memory,
        ws,
        &free.action_id,
        &free,
        Some(&free_decl),
        &effective,
        &empty_ctx(),
        store.pool(),
        now,
    )
    .await
    .unwrap();
    assert!(decision.is_auto_approved());
}

#[tokio::test]
async fn persisted_spend_is_isolated_by_worker() {
    let (store, memory, ws) = open_test_memory().await;
    let policy = enabled_policy(1_000_000);
    let effective = effective_from(policy.clone());
    let now = at(6, 12);
    worker_budget::record_charge(store.pool(), "seed-a", "worker-a", 1_000_000, now)
        .await
        .unwrap();

    let action = rec_for_worker("act-worker-b", "worker-b");
    seed(&memory, ws, &action).await;
    let decl = cost_decl(Some(1.0));
    let decision = maybe_auto_approve_with_pool(
        &memory,
        ws,
        &action.action_id,
        &action,
        Some(&decl),
        &effective,
        &empty_ctx(),
        store.pool(),
        now,
    )
    .await
    .unwrap();
    assert!(decision.is_auto_approved());
    assert_eq!(
        worker_budget::spend_today_for_worker(store.pool(), "worker-b", now)
            .await
            .unwrap(),
        1_000_000
    );
}

#[tokio::test]
async fn yesterday_spend_does_not_count_toward_today() {
    let (store, memory, ws) = open_test_memory().await;
    let policy = enabled_policy(1_000_000);
    let effective = effective_from(policy.clone());
    let today = at(6, 12);
    worker_budget::record_charge(
        store.pool(),
        "seed-yesterday",
        "worker-a",
        1_000_000,
        at(5, 12),
    )
    .await
    .unwrap();
    assert_eq!(
        worker_budget::spend_today_for_worker(store.pool(), "worker-a", today)
            .await
            .unwrap(),
        0
    );

    let action = rec_for_worker("act-today", "worker-a");
    seed(&memory, ws, &action).await;
    let decl = cost_decl(Some(1.0));
    let decision = maybe_auto_approve_with_pool(
        &memory,
        ws,
        &action.action_id,
        &action,
        Some(&decl),
        &effective,
        &empty_ctx(),
        store.pool(),
        today,
    )
    .await
    .unwrap();
    assert!(decision.is_auto_approved());
}
