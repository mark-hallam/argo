//! Cross-tenant regression coverage for sprint brief
//! `sprint-ws1-1b-migrate-cwd-resolver-tranche-2-notifications-worker-policies`.
//!
//! WS1-1b tranche 2 migrates two more user-scoped gateway route files off
//! the per-file cwd resolver (which resolved every signed-in user to the
//! gateway process's working directory — the seeded demo workspace) onto
//! the canonical `resolve_tenant::resolve_tenant_for_request` (JWT-session
//! tenant → explicit override → cwd):
//!
//! - `routes_notifications.rs` — the bell-popover notifications API. Before
//!   this, a signed-in user's `/api/v1/notifications` was scoped to the
//!   shared demo tenant, exposing other users' notification bodies.
//! - `routes_worker_policies.rs` — per-worker approval-policy config. Worker
//!   existence + the policy-change audit row were both resolved against the
//!   cwd tenant; the JWT tenant is the correct scope.
//!
//! ## Two complementary checks (no gateway boot)
//!
//! 1. **Data-isolation (public store API).** Provision two users A and B via
//!    `argo_identity::provision_for_user` — each gets its own tenant — and
//!    prove via the public `argo_store::notifications` API that a row written
//!    under A is visible ONLY under A, never under B, never under a
//!    separately-seeded demo tenant. This is the tenant property the
//!    JWT-tenant resolution unlocks end-to-end: once the route resolves to
//!    the user's JWT tenant, the tenant-scoped store query keeps the data
//!    isolated.
//!
//! 2. **Structural pins.** Assert the two migrated route files route tenant
//!    resolution through `resolve_tenant_for_request` and no longer carry a
//!    private cwd resolver. This mirrors the keystone's
//!    `tenant_resolve_from_session_jwt.rs` and tranche 1's
//!    `ws1_1b_billing_approvals_governance_tenant_resolve.rs`, which document
//!    WHY a structural assertion (not a boot→HTTP probe) is used for this
//!    test family: gateway-boot integration tests across this crate hit a
//!    pre-existing `GatewayCrashed` listener-signal flake on the build
//!    machine. A structural pin catches the same drift (a route silently
//!    reverting to the cwd resolver) without taking on that flake. The full
//!    JWT-cookie → tenant precedence (JWT wins over an `X-Tenant-Id` header)
//!    is exercised directly against the shared resolver in the crate-internal
//!    unit tests (`resolve_tenant_tests.rs`), the only place the `pub(super)`
//!    resolver is reachable; both routes now call that same entry point, so
//!    the precedence holds for them by construction and the structural pin
//!    guarantees the call.
//!
//! Note: `routes_worker_policies.rs` legitimately still reads
//! `std::env::current_dir` — NOT for tenant resolution, but for the
//! `.skycot/worker-policies/<worker_id>.toml` file path (a filesystem store
//! keyed to the gateway workspace dir, gated by the JWT-tenant worker lookup
//! first). The `std::env::current_dir` pin is therefore applied only to
//! `routes_notifications.rs`, whose sole `current_dir` use WAS the tenant
//! resolver.

use argo_identity::user::UserAccount;
use argo_identity::user_store::insert_user;
use argo_identity::{provision_for_user, ProvisionRequest};
use argo_store::notifications::{insert, list_for_principal, ListQuery, NotificationInsert};
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

async fn seed_notification(store: &SessionStore, tenant: &str, id: &str, body: &str) {
    insert(
        store.pool(),
        &NotificationInsert {
            id,
            tenant_id: tenant,
            principal_id: None,
            kind: "budget-approaching",
            body,
            created_at: 1_700_000_000,
            related_audit_id: None,
            severity: "info",
        },
    )
    .await
    .expect("insert notification");
}

/// A notification written under A's tenant is listed ONLY under A — never
/// under B's tenant, never under a separately-seeded demo tenant. This is the
/// isolation the route-level JWT-tenant resolution makes reachable for a
/// signed-in user (the route resolves to A's tenant; the tenant-scoped
/// `list_for_principal` query then refuses to surface B's or demo rows).
#[tokio::test]
async fn notifications_are_tenant_isolated_at_the_store_boundary() {
    let store = fresh_store().await;
    let tenant_a = provision(&store, "a@acme.test", "Acme Workspace").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta Workspace").await;
    assert_ne!(tenant_a, tenant_b, "each user must own a distinct tenant");

    // Stand in for the cwd-bound demo tenant the OLD resolver returned for
    // everyone — give it a notification so a cwd regression would be observable.
    let demo_tenant = TenantId::new();
    seed_notification(&store, &demo_tenant.to_string(), "nt-demo-1", "demo notice").await;

    // A gets a notification in its own tenant.
    seed_notification(&store, &tenant_a.to_string(), "nt-a-1", "A notice").await;

    let query = ListQuery {
        limit: Some(50),
        before: None,
        exclude_dismissed: true,
    };
    let a_page = list_for_principal(store.pool(), &tenant_a.to_string(), None, &query)
        .await
        .expect("list A");
    let b_page = list_for_principal(store.pool(), &tenant_b.to_string(), None, &query)
        .await
        .expect("list B");

    assert_eq!(a_page.items.len(), 1, "A sees exactly its own notification");
    assert_eq!(a_page.items[0].body, "A notice");
    assert!(
        b_page.items.is_empty(),
        "B must NOT see A's or the demo tenant's notification — got {:?}",
        b_page.items
    );
}

fn read_src(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Each migrated route resolves tenant through the canonical resolver and no
/// longer defines a private cwd `resolve_workspace_tenant`. A route silently
/// reverting to a private cwd resolver re-opens the cross-tenant exposure
/// this tranche closed; these pins fail on that drift.
#[test]
fn migrated_routes_use_canonical_resolver_and_drop_cwd_copies() {
    let must_call = [
        "src/handlers/gateway/routes_notifications.rs",
        "src/handlers/gateway/routes_worker_policies.rs",
    ];
    for rel in must_call {
        let src = read_src(rel);
        assert!(
            src.contains("resolve_tenant_for_request"),
            "{rel} no longer calls resolve_tenant_for_request — the WS1-1b \
             JWT-tenant resolution regressed back toward the cwd tenant."
        );
        assert!(
            !src.contains("fn resolve_workspace_tenant"),
            "{rel} still defines a private cwd `resolve_workspace_tenant` copy."
        );
    }

    // routes_notifications.rs's ONLY use of current_dir was the tenant
    // resolver; after migration it must read no cwd at all. (worker_policies
    // legitimately keeps current_dir for the per-worker policy file path, so
    // this stronger pin is intentionally NOT applied to it.)
    let notifications = read_src("src/handlers/gateway/routes_notifications.rs");
    assert!(
        !notifications.contains("std::env::current_dir"),
        "routes_notifications.rs still reads std::env::current_dir for tenant \
         resolution — the cwd-tenant bug the canonical resolver replaces."
    );
}

/// The canonical resolver module exists and exposes the single entry point
/// the migrated routes now share.
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
