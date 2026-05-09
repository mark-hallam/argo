//! End-to-end tests for `TenantQuotaStore::check_or_reject` —
//! the hard-enforcement gate added by
//! `claude-tenant-quota-enforcement-stricter`.
//!
//! Schema is shared with the existing observational tests
//! (`tenant_quota_tests.rs`); these tests exercise the new
//! `check_or_reject` surface and confirm the four acceptance
//! invariants:
//!
//! - under-quota success
//! - at-limit reject (off-by-one)
//! - reject body shape (kind/current/limit fields)
//! - re-check passes after admin raises the limit
//!
//! Tests against a fresh in-memory `SessionStore` so cargo can
//! run them in parallel without ordering pain.

use argo_store::tenant_quota::{QuotaPeriod, TenantQuotaStore, TenantQuotaUpdate};
use argo_store::tenant_quota_kinds::{QuotaActionKind, QuotaError};
use argo_store::SessionStore;
use chrono::{DateTime, TimeZone, Utc};
use sqlx::SqlitePool;

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

fn at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 9, 12, 0, 0).unwrap()
}

/// Seed one worker plus `count` `worker_budget` rows so the
/// tenant has a known `used_actions` total.
async fn seed_actions(pool: &SqlitePool, count: u64) {
    sqlx::query(
        "INSERT INTO worker
            (id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
             auto_approval_json, channels_json, created_at)
         VALUES ('w-1', 't-1', 'support', 'p-1', 'r-1', 'worker_self', '{}', '[]', 0)",
    )
    .execute(pool)
    .await
    .unwrap();
    for i in 0..count {
        sqlx::query(
            "INSERT INTO worker_budget \
             (tenant_id, worker_id, action_id, usd_micros, charged_at) \
             VALUES (x'00', 'w-1', ?, 1, ?)",
        )
        .bind(format!("a-{i}"))
        .bind(at().timestamp())
        .execute(pool)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn check_or_reject_under_quota_passes() {
    let store = store().await;
    seed_actions(store.pool(), 3).await;
    let quotas = TenantQuotaStore::new(store.pool());
    quotas
        .set_quota(
            "t-1",
            TenantQuotaUpdate {
                period: Some(QuotaPeriod::Daily),
                max_usd_micros: None,
                max_actions: Some(10),
                max_workers: None,
            },
            at(),
        )
        .await
        .unwrap();
    let res = quotas
        .check_or_reject("t-1", QuotaActionKind::Messages, 1, at())
        .await;
    assert!(res.is_ok(), "under-quota should pass: {res:?}");
}

#[tokio::test]
async fn check_or_reject_at_limit_rejects_off_by_one() {
    let store = store().await;
    seed_actions(store.pool(), 5).await;
    let quotas = TenantQuotaStore::new(store.pool());
    quotas
        .set_quota(
            "t-1",
            TenantQuotaUpdate {
                period: Some(QuotaPeriod::Daily),
                max_usd_micros: None,
                max_actions: Some(5),
                max_workers: None,
            },
            at(),
        )
        .await
        .unwrap();
    // delta=0 against equal usage/limit must NOT exceed.
    let allowed = quotas
        .check_or_reject("t-1", QuotaActionKind::Messages, 0, at())
        .await;
    assert!(allowed.is_ok(), "delta=0 should not exceed equal limit");
    // delta=1 against equal usage/limit MUST reject.
    let blocked = quotas
        .check_or_reject("t-1", QuotaActionKind::Messages, 1, at())
        .await;
    match blocked {
        Err(QuotaError::Exceeded {
            kind,
            current,
            limit,
        }) => {
            assert_eq!(kind, "messages");
            assert_eq!(current, 5);
            assert_eq!(limit, 5);
        }
        other => panic!("expected Exceeded, got {other:?}"),
    }
}

#[tokio::test]
async fn rejected_check_does_not_consume_quota() {
    let store = store().await;
    seed_actions(store.pool(), 5).await;
    let quotas = TenantQuotaStore::new(store.pool());
    quotas
        .set_quota(
            "t-1",
            TenantQuotaUpdate {
                period: Some(QuotaPeriod::Daily),
                max_usd_micros: None,
                max_actions: Some(5),
                max_workers: None,
            },
            at(),
        )
        .await
        .unwrap();
    let blocked = quotas
        .check_or_reject("t-1", QuotaActionKind::ForgeRuns, 1, at())
        .await;
    assert!(matches!(blocked, Err(QuotaError::Exceeded { .. })));
    // Reject is read-only — used_actions stays at 5.
    let summary = quotas.summary("t-1", at()).await.unwrap();
    assert_eq!(
        summary.usage.used_actions, 5,
        "rejected check must not write to worker_budget"
    );
}

#[tokio::test]
async fn admin_raises_limit_then_check_passes() {
    let store = store().await;
    seed_actions(store.pool(), 5).await;
    let quotas = TenantQuotaStore::new(store.pool());
    quotas
        .set_quota(
            "t-1",
            TenantQuotaUpdate {
                period: Some(QuotaPeriod::Daily),
                max_usd_micros: None,
                max_actions: Some(5),
                max_workers: None,
            },
            at(),
        )
        .await
        .unwrap();
    let blocked = quotas
        .check_or_reject("t-1", QuotaActionKind::SkillInvocations, 1, at())
        .await;
    assert!(
        matches!(blocked, Err(QuotaError::Exceeded { .. })),
        "expected Exceeded before admin raise: {blocked:?}"
    );
    quotas
        .set_quota(
            "t-1",
            TenantQuotaUpdate {
                period: None,
                max_usd_micros: None,
                max_actions: Some(20),
                max_workers: None,
            },
            at(),
        )
        .await
        .unwrap();
    let allowed = quotas
        .check_or_reject("t-1", QuotaActionKind::SkillInvocations, 1, at())
        .await;
    assert!(
        allowed.is_ok(),
        "post-raise check should pass: {allowed:?}"
    );
}
