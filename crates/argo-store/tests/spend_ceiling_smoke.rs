//! Integration smoke for `argo_store::spend_ceiling::check_ceiling`:
//! seed spend rows for a tenant + session, then read back the
//! `CeilingCheck` snapshot at three windows and assert the
//! `remaining` math against the configured ceilings.
//!
//! Mirrors the brief's "integration smoke: seed spend rows,
//! check_ceiling at 3 windows, assert remaining math" acceptance
//! criterion (Stream `mission-llm-budget-hard-ceiling`).
use argo_store::spend_ceiling::CeilingConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, BudgetWindow, SessionId, TenantId};
use chrono::Utc;

#[tokio::test]
async fn check_ceiling_round_trips_seeded_spend_rows() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    let tenant = TenantId::new();
    let session = SessionId::new();
    let now = Utc::now();

    // Seed three charges in the current session — all three windows
    // pick up the same total since the session is the most-recent
    // span and lives inside today/this-month.
    store
        .record_spend(
            tenant,
            session,
            now.timestamp_micros(),
            250_000,
            "openai",
            "gpt-4",
        )
        .await
        .unwrap();
    store
        .record_spend(
            tenant,
            session,
            now.timestamp_micros(),
            500_000,
            "openai",
            "gpt-4",
        )
        .await
        .unwrap();
    store
        .record_spend(
            tenant,
            session,
            now.timestamp_micros(),
            250_000,
            "anthropic",
            "claude",
        )
        .await
        .unwrap();
    // Total: 1_000_000 micros ($1.00) across all three windows.

    let config = CeilingConfig {
        per_session_usd_micros: Some(2_000_000), // $2.00 → 1.00 remaining
        per_day_usd_micros: Some(5_000_000),     // $5.00 → 4.00 remaining
        per_month_usd_micros: Some(10_000_000),  // $10.00 → 9.00 remaining
    };
    let check = store
        .check_ceiling(tenant, session, &config, now)
        .await
        .unwrap();

    assert_eq!(check.session_used_micros, 1_000_000);
    assert_eq!(check.day_used_micros, 1_000_000);
    assert_eq!(check.month_used_micros, 1_000_000);

    assert_eq!(check.session_remaining, Some(1_000_000));
    assert_eq!(check.day_remaining, Some(4_000_000));
    assert_eq!(check.month_remaining, Some(9_000_000));

    // Project a charge that fits under every window — passes.
    check.project(500_000).expect("$0.50 under all caps");

    // Project a charge that overshoots the session window only —
    // session is the tightest cap, so it fires first.
    let err = check
        .project(1_000_001)
        .expect_err("$1.000001 overshoots session by 1 micro");
    match err {
        ArgoError::BudgetExceeded { window, .. } => {
            assert_eq!(window, BudgetWindow::Session);
        }
        other => panic!("expected BudgetExceeded, got {other:?}"),
    }
}

#[tokio::test]
async fn check_ceiling_with_uncapped_config_returns_none_remaining() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();

    let tenant = TenantId::new();
    let session = SessionId::new();
    let now = Utc::now();

    store
        .record_spend(
            tenant,
            session,
            now.timestamp_micros(),
            999_000_000,
            "openai",
            "gpt-4",
        )
        .await
        .unwrap();

    let check = store
        .check_ceiling(tenant, session, &CeilingConfig::uncapped(), now)
        .await
        .unwrap();
    assert!(check.session_remaining.is_none());
    assert!(check.day_remaining.is_none());
    assert!(check.month_remaining.is_none());
    // Even a giant charge passes through when nothing is set.
    check.project(u64::MAX).expect("uncapped never blocks");
}
