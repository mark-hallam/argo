//! Cross-tenant regression coverage for the WS1 long-tail migration.
//!
//! Follow-up to the WS1 keystone (#2676) + tranches 1/2 (#2708/#2710):
//! the remaining user-scoped gateway route files are migrated off their
//! per-file cwd resolver (which resolved every signed-in user to the
//! gateway process's working directory — the seeded demo workspace) onto
//! the canonical `resolve_tenant::resolve_tenant_for_request`
//! (JWT-session tenant -> explicit override -> cwd fallback).
//!
//! Two complementary checks (no gateway boot — same rationale as the
//! tranche-1/2 tests: gateway-boot integration hits a pre-existing
//! listener-signal flake on the build machine, so a structural pin
//! catches the same drift without taking on that flake; the full
//! JWT-over-header precedence is exercised against the shared resolver in
//! the crate-internal `resolve_tenant_tests.rs`):
//!
//! 1. **Data-isolation at the store boundary for the flip-relevant duties
//!    path.** A schedule (duty) written under tenant A is listed ONLY
//!    under A, never under B, never under a cwd-stand-in demo tenant. This
//!    is the property the route-level JWT-tenant resolution makes reachable
//!    for a signed-in user — and it is the one long-tail file that feeds
//!    the autonomous flip, because the duty row's `tenant_id` (stamped by
//!    the migrated `routes_duties_helpers` create/manage route) is the
//!    tenant an auto-firing schedule later runs under.
//! 2. **Structural pins.** Every migrated long-tail route calls
//!    `resolve_tenant_for_request` and no longer defines a private cwd
//!    `resolve_workspace_tenant`. A route silently reverting to the cwd
//!    resolver re-opens the cross-tenant exposure; these pins fail on that
//!    drift. The list grows as each long-tail tranche lands.

use argo_identity::user::UserAccount;
use argo_identity::user_store::insert_user;
use argo_identity::{provision_for_user, ProvisionRequest, WorkerId};
use argo_orchestrator::{DutyRecord, DutyStore};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::PathBuf;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn provision(store: &SessionStore, email: &str, ws: &str) -> TenantId {
    let user = UserAccount::new(email.to_string());
    insert_user(store, &user).await.expect("insert user");
    provision_for_user(
        store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: ws,
            company_name: None,
        },
    )
    .await
    .expect("provision")
    .tenant_id
}

/// A schedule (duty) written under A's tenant is listed ONLY under A —
/// never under B's tenant, never under a separately-seeded demo/cwd
/// tenant. Once `routes_duties_helpers` resolves to A's JWT tenant, the
/// tenant-scoped `DutyStore::list_for_tenant` query keeps the schedule
/// isolated, and the autonomous fire path inherits the correct tenant
/// from the duty row.
#[tokio::test]
async fn duties_are_tenant_isolated_at_the_store_boundary() {
    let store = fresh_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme Workspace").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta Workspace").await;
    assert_ne!(tenant_a, tenant_b, "each user must own a distinct tenant");

    // Stand in for the cwd-bound demo tenant the OLD resolver returned for
    // everyone — give it a duty so a cwd regression would be observable.
    let demo_tenant = TenantId::new();
    let duties = DutyStore::new(Arc::clone(&store));

    duties
        .insert(&DutyRecord::new(
            demo_tenant,
            WorkerId::new(),
            "0 9 * * *",
            "draft_summary",
            "demo duty",
        ))
        .await
        .expect("insert demo duty");
    duties
        .insert(&DutyRecord::new(
            tenant_a,
            WorkerId::new(),
            "0 9 * * *",
            "draft_summary",
            "A duty",
        ))
        .await
        .expect("insert A duty");

    let a_list = duties.list_for_tenant(tenant_a).await.expect("list A");
    let b_list = duties.list_for_tenant(tenant_b).await.expect("list B");

    assert_eq!(a_list.len(), 1, "A sees exactly its own duty");
    assert_eq!(a_list[0].description, "A duty");
    assert!(
        b_list.is_empty(),
        "B must NOT see A's or the demo tenant's duty — got {b_list:?}"
    );
}

fn read_src(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every migrated long-tail route resolves tenant through the canonical
/// resolver and no longer defines a private cwd `resolve_workspace_tenant`.
/// Extended as each long-tail tranche lands.
#[test]
fn migrated_long_tail_routes_use_canonical_resolver() {
    let must_call = [
        // Tranche 1 — flip-relevant (duty tenant-stamping).
        "src/handlers/gateway/routes_duties_helpers.rs",
        // Tranche 2 — agent-draft routes (Grant GTM, Marcus/Nori marketing).
        "src/handlers/gateway/routes_gtm_draft_followup.rs",
        "src/handlers/gateway/routes_marketing_draft_response.rs",
    ];
    for rel in must_call {
        let src = read_src(rel);
        assert!(
            src.contains("resolve_tenant_for_request"),
            "{rel} no longer calls resolve_tenant_for_request — the WS1 \
             long-tail JWT-tenant resolution regressed toward the cwd tenant."
        );
        assert!(
            !src.contains("fn resolve_workspace_tenant"),
            "{rel} still defines a private cwd `resolve_workspace_tenant` copy."
        );
    }
}
