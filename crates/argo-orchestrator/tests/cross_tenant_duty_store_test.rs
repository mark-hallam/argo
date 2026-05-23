//! Surface 6 of the cross-tenant negative-test contract:
//! [`DutyStore`] id-scoped methods.
//!
//! Companion to
//! [`crates/argo-orchestrator/tests/cross_tenant_handoff_test.rs`]
//! (which pins Surfaces 1-5 — Handoff, RetryPolicy, DutyStore
//! `list_for_tenant`, and SLA). Split out as its own file because
//! the parent test would have exceeded the 500-LOC ceiling.
//!
//! Brief: `mission-orchestrator-duty-store-tenant-args`. Source of
//! the finding: the FINDINGS docstring in
//! `cross_tenant_handoff_test.rs` (now marked RESOLVED).
//!
//! Each test seeds a single duty under `tenant_a`, exercises
//! `tenant_b` against `tenant_a`'s `duty_id`, and pairs that response
//! with a genuinely nonexistent-id call to assert byte-identical
//! `Debug` repr. The byte-identical-shape bar comes from the
//! existence-leak pattern of #2068 ("just returns 404" is too weak;
//! cross-tenant must be indistinguishable from genuine-not-found).

use argo_identity::WorkerId;
use argo_orchestrator::{DutyRecord, DutyStore, FireDecision, SessionId};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let s = Arc::new(SessionStore::open(":memory:").await.unwrap());
    s.migrate().await.unwrap();
    s
}

/// Two-tenant DutyStore fixture: one duty in tenant_a, one fired-row
/// recorded against it so `sessions_for_duty` has something to find.
struct TwoDutyTenants {
    duties: DutyStore,
    tenant_a: TenantId,
    tenant_b: TenantId,
    duty_a: DutyRecord,
}

async fn two_duty_tenants() -> TwoDutyTenants {
    let store = fresh_store().await;
    let duties = DutyStore::new(store);
    duties.ensure_schema().await.unwrap();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let duty_a = DutyRecord::new(
        tenant_a,
        WorkerId::new(),
        "0 * * * * *",
        "ping",
        "tenant_a duty",
    );
    duties.insert(&duty_a).await.unwrap();
    // Seed one attribution row so sessions_for_duty has data to
    // exercise — the cross-tenant probe asks "does tenant_b see any
    // sessions for duty_a's id?". Without a seed, both the cross
    // and the nonexistent branches return Vec[] regardless.
    duties
        .link_fire(
            tenant_a,
            &duty_a.id,
            &SessionId::from_string(uuid::Uuid::now_v7().to_string()),
            Utc::now(),
            FireDecision::AutoApprove,
        )
        .await
        .unwrap();
    TwoDutyTenants {
        duties,
        tenant_a,
        tenant_b,
        duty_a,
    }
}

#[tokio::test]
async fn duty_get_cross_tenant_returns_none_identical_to_nonexistent_id() {
    let f = two_duty_tenants().await;

    let cross = f.duties.get(f.tenant_b, &f.duty_a.id).await;
    let nonexistent = f.duties.get(f.tenant_b, "no-such-id").await;

    assert_eq!(
        format!("{cross:?}"),
        format!("{nonexistent:?}"),
        "cross-tenant get must be indistinguishable from nonexistent"
    );
    assert!(matches!(cross, Ok(None)));

    // Sanity: tenant_a sees its own duty.
    assert!(f.duties.get(f.tenant_a, &f.duty_a.id).await.unwrap().is_some());
}

#[tokio::test]
async fn duty_mark_fired_cross_tenant_is_noop_identical_to_nonexistent_id() {
    let f = two_duty_tenants().await;
    let before = f
        .duties
        .get(f.tenant_a, &f.duty_a.id)
        .await
        .unwrap()
        .unwrap();
    let fired_at = Utc::now();

    let cross = f
        .duties
        .mark_fired(f.tenant_b, &f.duty_a.id, fired_at)
        .await;
    let nonexistent = f.duties.mark_fired(f.tenant_b, "no-such-id", fired_at).await;

    assert_eq!(
        format!("{cross:?}"),
        format!("{nonexistent:?}"),
        "cross-tenant mark_fired must be indistinguishable from nonexistent"
    );
    assert!(cross.is_ok());

    // Sanity: tenant_a's row was NOT silently mutated by tenant_b's
    // forged call.
    let after = f
        .duties
        .get(f.tenant_a, &f.duty_a.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.last_fired_at, before.last_fired_at);
}

#[tokio::test]
async fn duty_set_paused_cross_tenant_is_noop_identical_to_nonexistent_id() {
    let f = two_duty_tenants().await;
    let until = Some(Utc::now() + chrono::Duration::hours(2));

    let cross = f.duties.set_paused(f.tenant_b, &f.duty_a.id, until).await;
    let nonexistent = f.duties.set_paused(f.tenant_b, "no-such-id", until).await;

    assert_eq!(
        format!("{cross:?}"),
        format!("{nonexistent:?}"),
        "cross-tenant set_paused must be indistinguishable from nonexistent"
    );
    assert!(cross.is_ok());

    // Sanity: tenant_a's row was NOT silently paused by tenant_b.
    let after = f
        .duties
        .get(f.tenant_a, &f.duty_a.id)
        .await
        .unwrap()
        .unwrap();
    assert!(after.paused_until.is_none());
}

#[tokio::test]
async fn duty_link_fire_cross_tenant_is_noop_identical_to_nonexistent_id() {
    let f = two_duty_tenants().await;
    let before = f
        .duties
        .sessions_for_duty(f.tenant_a, &f.duty_a.id)
        .await
        .unwrap();
    let new_session = SessionId::from_string(uuid::Uuid::now_v7().to_string());
    let fired_at = Utc::now();

    let cross = f
        .duties
        .link_fire(
            f.tenant_b,
            &f.duty_a.id,
            &new_session,
            fired_at,
            FireDecision::AutoApprove,
        )
        .await;
    let nonexistent = f
        .duties
        .link_fire(
            f.tenant_b,
            "no-such-id",
            &new_session,
            fired_at,
            FireDecision::AutoApprove,
        )
        .await;

    assert_eq!(
        format!("{cross:?}"),
        format!("{nonexistent:?}"),
        "cross-tenant link_fire must be indistinguishable from nonexistent"
    );
    assert!(cross.is_ok());

    // Sanity: tenant_a's attribution row count did NOT grow as a
    // side-effect of tenant_b's forged call.
    let after = f
        .duties
        .sessions_for_duty(f.tenant_a, &f.duty_a.id)
        .await
        .unwrap();
    assert_eq!(after.len(), before.len());
}

#[tokio::test]
async fn duty_sessions_for_duty_cross_tenant_returns_empty_identical_to_nonexistent_id() {
    let f = two_duty_tenants().await;

    let cross = f.duties.sessions_for_duty(f.tenant_b, &f.duty_a.id).await;
    let nonexistent = f.duties.sessions_for_duty(f.tenant_b, "no-such-id").await;

    assert_eq!(
        format!("{cross:?}"),
        format!("{nonexistent:?}"),
        "cross-tenant sessions_for_duty must be indistinguishable from nonexistent"
    );
    assert_eq!(cross.unwrap().len(), 0);

    // Sanity: tenant_a sees its own attribution rows.
    let owned = f
        .duties
        .sessions_for_duty(f.tenant_a, &f.duty_a.id)
        .await
        .unwrap();
    assert_eq!(owned.len(), 1);
}
