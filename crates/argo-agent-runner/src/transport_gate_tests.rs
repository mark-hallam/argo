//! Behavioral tests for the hard `[budget.ceiling]` choke point
//! ([`StoreBudgetEnforcer`] + [`TransportGate`]). These pin acceptance
//! criteria 2 + 3 of `launch-wire-spendgate-llm-cost-ceiling`:
//!
//! * a tenant at its ceiling gets the structured refusal + an audit row,
//!   and spend does NOT increment past the cap;
//! * the anonymous / pre-auth path reserves each allowed call against a
//!   stable capped tenant so the day/month window actually accumulates.
use super::*;
use argo_store::spend_ceiling::CeilingConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, BudgetWindow, SessionId, TenantId};
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

#[test]
fn anonymous_tenant_id_is_stable_and_non_nil() {
    // Stability is load-bearing: a fresh-random id per call would give every
    // anonymous request a fresh (empty) day window, so the cap would never
    // bite. Two calls MUST be equal, and it must not be the nil tenant.
    assert_eq!(anonymous_tenant_id(), anonymous_tenant_id());
    assert_ne!(anonymous_tenant_id(), TenantId(uuid::Uuid::nil()));
}

#[tokio::test]
async fn enforcer_blocks_at_day_ceiling_and_writes_audit() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let session = SessionId::new();
    // Fill the day window to exactly the ceiling.
    let day_ceiling = 50_000;
    store
        .record_spend(
            tenant,
            session,
            chrono::Utc::now().timestamp_micros(),
            day_ceiling,
            "p",
            "m",
        )
        .await
        .expect("seed spend");

    let config = CeilingConfig {
        per_session_usd_micros: None,
        per_day_usd_micros: Some(day_ceiling),
        per_month_usd_micros: None,
    };
    let enforcer = StoreBudgetEnforcer::new(store.clone(), tenant, session, config, false);

    // Any positive projection now overshoots remaining (== 0) → refusal.
    let err = enforcer.enforce(1).await.expect_err("must refuse at ceiling");
    match err {
        ArgoError::BudgetExceeded { window, .. } => assert_eq!(window, BudgetWindow::Day),
        other => panic!("expected BudgetExceeded, got {other:?}"),
    }

    // The block is audited (never a silent failure).
    let rows = store
        .list_audit_for_session(session)
        .await
        .expect("read audit");
    assert!(
        rows.iter().any(|r| r.kind == "budget.ceiling_blocked"),
        "a budget.ceiling_blocked row must be written on a block; got {:?}",
        rows.iter().map(|r| &r.kind).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn enforcer_allows_with_headroom_and_read_only_does_not_record() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let session = SessionId::new();
    let config = CeilingConfig {
        per_session_usd_micros: None,
        per_day_usd_micros: Some(1_000_000),
        per_month_usd_micros: None,
    };
    // reserve = false (the authed Session path): allow + record NOTHING (the
    // turn loop's record_turn_spend is the single writer).
    let enforcer = StoreBudgetEnforcer::new(store.clone(), tenant, session, config, false);
    enforcer.enforce(40_000).await.expect("allowed under ceiling");

    let check = store
        .check_ceiling(tenant, session, &config, chrono::Utc::now())
        .await
        .expect("check");
    assert_eq!(
        check.day_used_micros, 0,
        "read-only gate must not record any spend"
    );
}

#[tokio::test]
async fn reserve_accumulates_and_caps_anonymous_spend() {
    let store = mem_store().await;
    let tenant = anonymous_tenant_id();
    let session = SessionId::new();
    let config = CeilingConfig {
        per_session_usd_micros: None,
        per_day_usd_micros: Some(100_000),
        per_month_usd_micros: None,
    };
    // reserve = true (the anonymous gateway path): each allowed call records
    // its projected cost so the shared day window accumulates toward the cap.
    let enforcer = StoreBudgetEnforcer::new(store.clone(), tenant, session, config, true);

    enforcer.enforce(40_000).await.expect("1st allowed (40k/100k)");
    enforcer.enforce(40_000).await.expect("2nd allowed (80k/100k)");
    // 80k used, 20k remaining; a 40k projection overshoots → refusal. This is
    // the open-wallet close: total anonymous spend is bounded.
    let err = enforcer
        .enforce(40_000)
        .await
        .expect_err("3rd refused (would be 120k > 100k)");
    assert!(matches!(err, ArgoError::BudgetExceeded { .. }));

    let check = store
        .check_ceiling(tenant, session, &config, chrono::Utc::now())
        .await
        .expect("check");
    assert_eq!(
        check.day_used_micros, 80_000,
        "exactly the two allowed reservations accumulated; the blocked call recorded nothing"
    );
}
