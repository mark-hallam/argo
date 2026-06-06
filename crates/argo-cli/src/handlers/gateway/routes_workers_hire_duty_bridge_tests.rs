//! Tests for the hire → scheduler `role_duty` bridge (UF Phase-B · B1).
//!
//! Drives `bridge_hire_duties` directly against a `:memory:` store and
//! asserts the rows it writes are exactly what the scheduler's own
//! reader (`DutyStore::list_for_tenant`) returns — including the
//! enabled-only filter and tenant isolation. Calling the bridge unit
//! directly (rather than the full `hire_response` HTTP path) keeps the
//! tenant-isolation assertion crisp: the cwd-derived hire path resolves
//! a single tenant, so two distinct tenants are easiest to express by
//! passing explicit `TenantId`s here.

use super::bridge_hire_duties;
use argo_identity::{HireDraft, WorkerId};
use argo_orchestrator::DutyStore;
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::json;
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// A draft with one enabled duty and one disabled duty, mirroring the
/// SPA camelCase hire body shape.
fn draft_with_enabled_and_disabled() -> HireDraft {
    serde_json::from_value(json!({
        "suggestedName": "Maya",
        "charter": "Own outbound marketplace follow-up end-to-end for B2B customers.",
        "suggestedVertical": "gtm-engineer",
        "approvalPolicy": {"level": "balanced"},
        "duties": [
            {
                "cron": "0 9 * * 1-5",
                "entryAction": "follow_up",
                "description": "Send approved follow-up.",
                "defaultEnabled": true,
            },
            {
                "cron": "0 18 * * 5",
                "entryAction": "weekly_digest",
                "description": "Disabled at hire; must NOT reach the scheduler.",
                "defaultEnabled": false,
            },
        ],
        "tagline": "GTM follow-up persona",
    }))
    .expect("draft parses")
}

#[tokio::test]
async fn bridge_writes_only_enabled_duties_listable_by_scheduler() {
    let store = mem_store().await;
    let tenant = TenantId::new();
    let worker = WorkerId::new();
    let draft = draft_with_enabled_and_disabled();

    let written = bridge_hire_duties(&store, tenant, worker, &draft).await;
    assert_eq!(written, 1, "only the one enabled duty should bridge");

    let listed = DutyStore::new(Arc::clone(&store))
        .list_for_tenant(tenant)
        .await
        .expect("list role_duty");
    assert_eq!(listed.len(), 1, "scheduler sees exactly the enabled duty");
    let row = &listed[0];
    assert_eq!(row.worker_id, worker, "duty bound to the hired worker");
    assert_eq!(row.tenant_id, tenant, "duty bound to the hiring tenant");
    assert_eq!(row.cron, "0 9 * * 1-5");
    assert_eq!(row.entry_action, "follow_up");
    assert!(row.enabled, "bridged duty lands enabled (listable)");
}

#[tokio::test]
async fn bridge_is_tenant_isolated() {
    let store = mem_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let worker_a = WorkerId::new();
    let draft = draft_with_enabled_and_disabled();

    bridge_hire_duties(&store, tenant_a, worker_a, &draft).await;

    let duty_store = DutyStore::new(Arc::clone(&store));
    let a_rows = duty_store
        .list_for_tenant(tenant_a)
        .await
        .expect("list A");
    let b_rows = duty_store
        .list_for_tenant(tenant_b)
        .await
        .expect("list B");
    assert_eq!(a_rows.len(), 1, "A sees its own hire's duty");
    assert!(
        b_rows.is_empty(),
        "B must NOT see A's hire's duty (tenant isolation)"
    );
}
