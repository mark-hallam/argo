//! Integration coverage for sprint brief
//! `sprint-ws1-tenant-resolve-from-session-jwt-not-cwd`.
//!
//! The keystone fix: every signed-in user's read routes must resolve to
//! the user-bound tenant carried in their `__Host-session` JWT, not the
//! gateway process's cwd (the seeded demo workspace). A stranger who
//! signs up must land in their own EMPTY private workspace.
//!
//! ## Two complementary checks (no gateway boot)
//!
//! 1. **Data-isolation (public API).** Provision two users A and B via
//!    `argo_identity::provision_for_user` — each gets its own tenant —
//!    and prove via the public `team_roster` store API that A's roster
//!    is empty, B's roster is empty, and a separately-seeded demo
//!    tenant's roster is NOT visible under A. This is the data property
//!    the JWT-tenant resolution unlocks end-to-end.
//!
//! 2. **Structural pins.** Assert the migrated core read-route files
//!    route tenant resolution through the canonical
//!    `resolve_tenant::resolve_tenant_for_request` and no longer carry a
//!    private cwd `resolve_workspace_tenant` copy. This mirrors the
//!    canonical pattern in `gateway_uuid_routes_tenant_ownership.rs`,
//!    which documents WHY a structural assertion (not a boot→HTTP probe)
//!    is used for this test family: gateway-boot integration tests
//!    across this crate hit a pre-existing `GatewayCrashed`
//!    listener-signal flake on the build machine. A structural assertion
//!    catches the same drift (a route silently reverting to the cwd
//!    resolver) without taking on that flake. The full JWT-cookie →
//!    tenant resolution is exercised directly in the crate-internal unit
//!    tests (`resolve_tenant_tests.rs`), which is the only place the
//!    `pub(super)` resolver is reachable.

use argo_identity::user::UserAccount;
use argo_identity::user_store::insert_user;
use argo_identity::{provision_for_user, ProvisionRequest};
use argo_store::team_roster::list_teams;
use argo_store::team_roster_mutations::create_team;
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

/// Provision two users → distinct tenants, each with an EMPTY roster;
/// a separately-seeded demo tenant's team never appears under A or B.
#[tokio::test]
async fn provisioned_users_get_distinct_empty_private_workspaces() {
    let store = fresh_store().await;

    let tenant_a = provision(&store, "a@acme.test", "Acme Workspace").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta Workspace").await;
    assert_ne!(tenant_a, tenant_b, "each user must own a distinct tenant");

    // Stand in for the cwd-bound demo tenant the OLD resolver returned
    // for everyone: seed it with a team so "leaks into demo data" would
    // be observable.
    let demo_tenant = TenantId::new();
    create_team(store.pool(), demo_tenant, "Demo Reception", None)
        .await
        .expect("seed demo team");

    let demo_roster = list_teams(store.pool(), demo_tenant).await.expect("demo roster");
    assert_eq!(demo_roster.len(), 1, "demo tenant has its seeded team");

    // The fix's whole point: a fresh signed-up user sees an EMPTY
    // roster scoped to THEIR tenant, never the demo tenant's rows.
    let roster_a = list_teams(store.pool(), tenant_a).await.expect("roster A");
    let roster_b = list_teams(store.pool(), tenant_b).await.expect("roster B");
    assert!(roster_a.is_empty(), "fresh user A's private workspace is empty");
    assert!(roster_b.is_empty(), "fresh user B's private workspace is empty");
}

fn read_src(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every migrated core read-route file routes tenant resolution through
/// the canonical resolver. A route silently reverting to a private cwd
/// `resolve_workspace_tenant` copy re-opens the keystone bug; this pins
/// against that drift.
#[test]
fn core_read_routes_use_canonical_resolver_and_drop_cwd_copies() {
    // (file, must-reference-canonical-resolver)
    let direct = [
        "src/handlers/gateway/routes_team_list.rs",
        "src/handlers/gateway/routes_worker_overview.rs",
        "src/handlers/gateway/routes_worker_detail.rs",
        "src/handlers/gateway/routes_skills.rs",
        "src/handlers/gateway/usage_endpoint.rs",
        "src/handlers/gateway/routes_approvals_json.rs",
        "src/handlers/gateway/routes_approvals_json_seeded.rs",
        // audit feed's default scope now flows through the resolver too.
        "src/handlers/gateway/audit_query.rs",
    ];
    for rel in direct {
        let src = read_src(rel);
        assert!(
            src.contains("resolve_tenant_for_request"),
            "{rel} no longer calls resolve_tenant_for_request — the WS1 \
             JWT-tenant resolution regressed back toward the cwd tenant."
        );
    }

    // The 6 originally-listed files must no longer DEFINE a private
    // cwd resolver (`fn resolve_workspace_tenant` / `_id`). The canonical
    // module in resolve_tenant.rs is the single home now.
    let no_local_copy = [
        "src/handlers/gateway/routes_team_list.rs",
        "src/handlers/gateway/routes_worker_overview.rs",
        "src/handlers/gateway/routes_worker_detail.rs",
        "src/handlers/gateway/routes_skills.rs",
        "src/handlers/gateway/usage_endpoint.rs",
        "src/handlers/gateway/routes_approvals_json.rs",
    ];
    for rel in no_local_copy {
        let src = read_src(rel);
        assert!(
            !src.contains("fn resolve_workspace_tenant"),
            "{rel} still defines a private cwd `resolve_workspace_tenant` \
             copy — migrate its callers to resolve_tenant_for_request."
        );
        assert!(
            !src.contains("std::env::current_dir"),
            "{rel} still reads std::env::current_dir for tenant resolution \
             — that is the cwd-tenant bug the canonical resolver replaces."
        );
    }
}

/// The canonical resolver module exists and declares the single entry
/// point with the documented 3-tier precedence.
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
