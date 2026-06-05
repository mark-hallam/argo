//! Cross-tenant regression coverage for sprint brief
//! `sprint-ws1-1b-migrate-cwd-resolver-tranche-1-billing-approvals-governance`.
//!
//! WS1-1b tranche 1 migrates the 3 highest-sensitivity gateway routes
//! that still resolved tenant from the gateway process's cwd (the seeded
//! demo workspace) onto the canonical
//! `resolve_tenant::resolve_tenant_for_request` (JWT-session tenant →
//! explicit override → cwd). Before this, a signed-in user hitting
//! `/billing/checkout`, `/approvals/bundles`, or `/governance/readiness`
//! was scoped to the shared demo tenant — a cross-tenant exposure
//! (FINANCIAL on billing).
//!
//! ## Two complementary checks (no gateway boot)
//!
//! 1. **Data-isolation (public API).** Provision two users A and B via
//!    `argo_identity::provision_for_user` — each gets its own tenant —
//!    and prove via the public `approval_bundle` store API that a bundle
//!    created under A is visible ONLY under A, never under B. This is the
//!    tenant property the JWT-tenant resolution unlocks end-to-end: once
//!    the route resolves to the user's JWT tenant, the tenant-scoped
//!    store query keeps the data isolated.
//!
//! 2. **Structural pins.** Assert the 3 migrated route files route tenant
//!    resolution through `resolve_tenant_for_request` and no longer carry
//!    a private cwd resolver. This mirrors the keystone's
//!    `tenant_resolve_from_session_jwt.rs`, which documents WHY a
//!    structural assertion (not a boot→HTTP probe) is used for this test
//!    family: gateway-boot integration tests across this crate hit a
//!    pre-existing `GatewayCrashed` listener-signal flake on the build
//!    machine. A structural pin catches the same drift (a route silently
//!    reverting to the cwd resolver) without taking on that flake. The
//!    full JWT-cookie → tenant precedence (JWT wins over an `X-Tenant-Id`
//!    header) is exercised directly against the shared resolver in the
//!    crate-internal unit tests (`resolve_tenant_tests.rs`), the only
//!    place the `pub(super)` resolver is reachable; all 3 routes now call
//!    that same entry point, so the precedence holds for them by
//!    construction and the structural pin guarantees the call.

use argo_identity::user::UserAccount;
use argo_identity::user_store::insert_user;
use argo_identity::{provision_for_user, ProvisionRequest};
use argo_store::approval_bundle::{create_bundle, list_pending_bundles, NewBundle};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::PathBuf;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    store
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

/// A bundle created under A's tenant is listed ONLY under A — never under
/// B's tenant, never under a separately-seeded demo tenant. This is the
/// isolation the route-level JWT-tenant resolution makes reachable for a
/// signed-in user (the route resolves to A's tenant; the tenant-scoped
/// `list_pending_bundles` query then refuses to surface B's or demo rows).
#[tokio::test]
async fn approval_bundles_are_tenant_isolated_at_the_store_boundary() {
    let store = fresh_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme Workspace").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta Workspace").await;
    assert_ne!(tenant_a, tenant_b, "each user must own a distinct tenant");

    // Stand in for the cwd-bound demo tenant the OLD resolver returned for
    // everyone — give it a bundle so a cwd regression would be observable.
    let demo_tenant = TenantId::new();
    create_bundle(
        store.pool(),
        &NewBundle {
            tenant_id: demo_tenant,
            label: "demo bundle",
            member_approval_ids: vec!["demo-action-1".to_string()],
        },
    )
    .await
    .expect("seed demo bundle");

    // A creates a bundle in its own tenant.
    create_bundle(
        store.pool(),
        &NewBundle {
            tenant_id: tenant_a,
            label: "A bundle",
            member_approval_ids: vec!["a-action-1".to_string()],
        },
    )
    .await
    .expect("seed A bundle");

    let a_bundles = list_pending_bundles(store.pool(), tenant_a, 50)
        .await
        .expect("list A");
    let b_bundles = list_pending_bundles(store.pool(), tenant_b, 50)
        .await
        .expect("list B");

    assert_eq!(a_bundles.len(), 1, "A sees exactly its own bundle");
    assert_eq!(a_bundles[0].label, "A bundle");
    assert!(
        b_bundles.is_empty(),
        "B must NOT see A's or the demo tenant's bundle — got {b_bundles:?}"
    );
}

fn read_src(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Each migrated route resolves tenant through the canonical resolver. A
/// route silently reverting to a private cwd resolver re-opens the
/// cross-tenant exposure this tranche closed; these pins fail on that
/// drift.
#[test]
fn migrated_routes_use_canonical_resolver_and_drop_cwd_copies() {
    let must_call = [
        "src/handlers/gateway/billing.rs",
        "src/handlers/gateway/routes_approval_bundles.rs",
        "src/handlers/gateway/routes_governance.rs",
    ];
    for rel in must_call {
        let src = read_src(rel);
        assert!(
            src.contains("resolve_tenant_for_request"),
            "{rel} no longer calls resolve_tenant_for_request — the WS1-1b \
             JWT-tenant resolution regressed back toward the cwd tenant."
        );
    }

    // billing.rs + routes_approval_bundles.rs defined a private cwd
    // `resolve_workspace_tenant`; both must be gone, and neither may read
    // `std::env::current_dir` for tenant resolution any more.
    let no_cwd_resolver = [
        "src/handlers/gateway/billing.rs",
        "src/handlers/gateway/routes_approval_bundles.rs",
    ];
    for rel in no_cwd_resolver {
        let src = read_src(rel);
        assert!(
            !src.contains("fn resolve_workspace_tenant"),
            "{rel} still defines a private cwd `resolve_workspace_tenant` copy."
        );
        assert!(
            !src.contains("std::env::current_dir"),
            "{rel} still reads std::env::current_dir for tenant resolution \
             — the cwd-tenant bug the canonical resolver replaces."
        );
    }

    // routes_governance.rs defined a private async `resolve_tenant`; it
    // must be gone. (Its `capability_summary` legitimately still reads
    // current_dir for workspace-independent role-preset names, so the
    // std::env::current_dir pin above is intentionally NOT applied here.)
    let governance = read_src("src/handlers/gateway/routes_governance.rs");
    assert!(
        !governance.contains("async fn resolve_tenant("),
        "routes_governance.rs still defines a private cwd `resolve_tenant` \
         copy — migrate it to resolve_tenant_for_request."
    );
}

/// The canonical resolver module exists and exposes the single entry
/// point the migrated routes now share.
#[test]
fn canonical_resolver_module_present() {
    let src = read_src("src/handlers/gateway/resolve_tenant.rs");
    assert!(
        src.contains("pub(super) async fn resolve_tenant_for_request"),
        "resolve_tenant.rs must expose resolve_tenant_for_request"
    );
    assert!(
        src.contains("decode_session_cookie"),
        "resolver must read the __Host-session JWT (decode_session_cookie)"
    );
}
