//! Hard-cap enforcement tests for
//! `pricing-wire-overage-report-and-cap-enforcement` (part b).
//!
//! Proves the per-tenant hard spend cap (`tenant_quota.max_usd_micros`)
//! actually HALTS usage at the dispatch choke point — including the two
//! paths the prior AutoApprove-only check left open:
//!   1. a zero-estimate action while the tenant is exactly at its cap, and
//!   2. an autonomous-safe (`requires_approval == false`) action, which
//!      never entered the AutoApprove branch at all.

use super::*;
use argo_identity::worker_budget;
use argo_store::tenant_quota::{TenantQuotaStore, TenantQuotaUpdate};

/// Insert a minimal `worker` row so `tenant_for_worker` and the
/// usage join resolve `worker_id → tenant_id`.
async fn seed_worker(store: &SessionStore, worker_id: &str, tenant_id: &str) {
    sqlx::query(
        "INSERT INTO worker \
         (id, tenant_id, vertical_id, persona_id, role_id, memory_scope, \
          auto_approval_json, channels_json, created_at) \
         VALUES (?, ?, '', '', '', '', '{}', '{}', 0)",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .execute(store.pool())
    .await
    .unwrap();
}

/// Pin the tenant's USD spend to `at_micros` via the live `worker_budget`
/// meter (the same ledger `has_headroom_for` reads), and set the cap.
async fn seed_tenant_at_spend(
    store: &SessionStore,
    worker_id: &str,
    tenant_id: &str,
    cap_micros: u64,
    used_micros: u64,
    now: chrono::DateTime<Utc>,
) {
    seed_worker(store, worker_id, tenant_id).await;
    TenantQuotaStore::new(store.pool())
        .set_quota(
            tenant_id,
            TenantQuotaUpdate {
                max_usd_micros: Some(cap_micros),
                ..TenantQuotaUpdate::default()
            },
            now,
        )
        .await
        .unwrap();
    if used_micros > 0 {
        worker_budget::record_charge(store.pool(), "seed-charge", worker_id, used_micros, now)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn zero_estimate_action_is_blocked_when_tenant_is_at_cap() {
    // Closes hole #2: a tenant exactly at its USD cap must not be able to
    // dispatch a no-budget (estimate == 0) action.
    let (store, _memory, _ws) = open_test_memory().await;
    let now = at(6, 12);
    seed_tenant_at_spend(&store, "worker-cap", "tenant-cap", 1_000_000, 1_000_000, now).await;

    let err = ensure_tenant_quota_allows(store.pool(), "worker-cap", 0, now)
        .await
        .expect_err("at-cap tenant must be stopped even for a 0-estimate action");
    assert!(
        err.to_string().contains("max_usd"),
        "expected the USD cap to be the blocking reason, got: {err}"
    );
}

#[tokio::test]
async fn below_cap_zero_estimate_action_is_allowed() {
    // Regression guard: the `.max(1)` headroom reservation must not block
    // a tenant that still has room.
    let (store, _memory, _ws) = open_test_memory().await;
    let now = at(6, 12);
    seed_tenant_at_spend(&store, "worker-room", "tenant-room", 1_000_000, 500_000, now).await;

    ensure_tenant_quota_allows(store.pool(), "worker-room", 0, now)
        .await
        .expect("a below-cap tenant keeps dispatching");
}

#[tokio::test]
async fn dispatch_gate_stops_autonomous_safe_action_at_cap() {
    // Closes hole #1: an autonomous-safe action (`requires_approval ==
    // false`) never enters the AutoApprove branch, so the prior check
    // missed it entirely. The dispatch-level gate must still halt it.
    let (store, memory, ws) = open_test_memory().await;
    let now = at(6, 12);
    seed_tenant_at_spend(&store, "worker-auto", "tenant-auto", 1_000_000, 1_000_000, now).await;

    let mut action = rec("act-autonomous", "low", false);
    action.payload = serde_json::json!({ "worker_id": "worker-auto" });
    seed(&memory, ws, &action).await;

    let err = ensure_tenant_cap_allows_dispatch(&memory, ws, store.pool(), &action.action_id, now)
        .await
        .expect_err("autonomous-safe action must be stopped at the tenant cap");
    assert!(
        err.to_string().contains("max_usd"),
        "expected the USD cap to be the blocking reason, got: {err}"
    );
}

#[tokio::test]
async fn gateway_inline_approval_dispatch_is_stopped_at_cap() {
    // The gateway /approvals approve path dispatches via
    // `execute_approved_action_inline`, NOT `worker_actions_dispatch::execute`
    // — found in the #2870 adversarial review. Pin that this path runs the
    // same dispatch-time cap gate. The cap fires BEFORE executor resolution,
    // so the bogus vertical below would only be reached on a regression
    // (which would surface as a "not loadable" error, failing the assert).
    let (store, memory, ws) = open_test_memory().await;
    // Real clock, not the fixed `at()` harness time: the production path
    // under test calls `Utc::now()` internally, so the seeded spend must
    // land in the CURRENT day/month windows to consume the cap headroom.
    let now = Utc::now();
    seed_tenant_at_spend(&store, "worker-inline", "tenant-inline", 1_000_000, 1_000_000, now)
        .await;

    let mut action = rec("act-inline-approved", "low", true);
    action.payload = serde_json::json!({ "worker_id": "worker-inline" });
    seed(&memory, ws, &action).await;

    let err = crate::handlers::work_dispatch_executor_inline::execute_approved_action_inline(
        &store,
        &memory,
        ws,
        argo_types::TenantId::new(),
        "no-such-vertical",
        &action,
    )
    .await
    .expect_err("an at-cap tenant must be stopped on the gateway inline approve path");
    assert!(
        err.to_string().contains("max_usd"),
        "expected the USD cap to be the blocking reason (not vertical resolve), got: {err}"
    );
}
