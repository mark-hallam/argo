//! Loop-closure integration coverage for sprint brief
//! `sprint-ws1-2-wire-writes-to-session-jwt-tenant`.
//!
//! The keystone (#2676) migrated the gateway READ routes to resolve the
//! tenant from the `__Host-session` JWT instead of the gateway process's
//! cwd. This brief migrates the WRITE paths (onboarding **first-hire**
//! and worker **run-now**) to the SAME canonical resolver
//! (`resolve_tenant::resolve_tenant_for_request`) so a signed-in user's
//! mutation lands in their own tenant — the one their JWT-aware reads
//! query — instead of the shared cwd/demo tenant. Before this fix a
//! stranger could hire a teammate (write → cwd tenant) and then see an
//! EMPTY `/team` (read → their own JWT tenant). That broken loop is the
//! Friday bar this brief closes.
//!
//! ## Two complementary checks (no gateway boot)
//!
//! This mirrors the seeding + assertion style of
//! `tenant_resolve_from_session_jwt.rs`, which documents WHY a structural
//! pin + a public-store data check is used instead of a boot→HTTP probe:
//! the gateway-boot integration tests in this crate hit a pre-existing
//! `GatewayCrashed` listener-signal flake on the build machine, and the
//! resolver itself is `pub(super)` (reachable only from the crate-internal
//! unit tests in `resolve_tenant_tests.rs`, where the full JWT-cookie →
//! tenant path is exercised directly).
//!
//! 1. **Data loop-closure (public API).** Provision two users A and B —
//!    each gets its own tenant — then commit a worker under A's tenant via
//!    the SAME public `IdentityStore::create_worker_from_draft` the
//!    first-hire handler calls. Assert A's `/api/v1/team` read
//!    (`list_workers(tenant_a)`) sees the hire, B's does NOT, and the
//!    cwd/demo tenant does NOT. This is the data property the
//!    write-side JWT-tenant resolution unlocks end-to-end.
//!
//! 2. **Structural pins.** Assert the migrated write-route files route
//!    tenant resolution through the canonical resolver and no longer
//!    cwd-resolve the write tenant (the `cwd_write_tenant` shim is gone;
//!    `routes_worker_run.rs` no longer reads `std::env::current_dir`). A
//!    route silently reverting to the cwd tenant re-opens the broken
//!    loop; this pins against that drift.

use argo_identity::user::UserAccount;
use argo_identity::user_store::insert_user;
use argo_identity::{
    provision_for_user, ApprovalPolicyChoice, HireDraft, HireDutyDraft, IdentityStore,
    ProvisionRequest,
};
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

/// A minimal valid hire draft — same shape `first_hire_response` builds
/// before forwarding to `create_worker_from_draft`.
fn sample_hire(name: &str) -> HireDraft {
    HireDraft {
        suggested_name: name.to_string(),
        charter: "Own outbound customer follow-up end-to-end.".into(),
        suggested_vertical: "marketplace-seller".into(),
        approval_policy: ApprovalPolicyChoice {
            level: "balanced".into(),
        },
        duties: vec![HireDutyDraft {
            cron: "0 9 * * 1-5".into(),
            entry_action: "follow_up".into(),
            description: "Send approved follow-up.".into(),
            default_enabled: true,
        }],
        voice_profile: None,
        tagline: Some("Marketplace seller persona".into()),
        genesis_plan_id: None,
        fs_root: None,
    }
}

/// The load-bearing loop-closure assertion: a hire written under user A's
/// (JWT == provision) tenant is visible to A's `/team` read, invisible to
/// a different user B, and never lands in the cwd/demo tenant.
#[tokio::test]
async fn first_hire_write_lands_in_users_tenant_not_cwd() {
    let store = fresh_store().await;

    let tenant_a = provision(&store, "a@acme.test", "Acme Workspace").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta Workspace").await;
    assert_ne!(tenant_a, tenant_b, "each user must own a distinct tenant");

    // Stand in for the gateway cwd/demo tenant the OLD `cwd_write_tenant`
    // shim wrote every signed-in user's hire to. A's hire must NOT appear
    // here.
    let demo_tenant = TenantId::new();

    let identities = IdentityStore::new(&store);

    // The first-hire WRITE — same store call the migrated handler makes,
    // now scoped to A's tenant (what `resolve_tenant_for_request` returns
    // for A's JWT) rather than the cwd tenant.
    let worker_id = identities
        .create_worker_from_draft(&sample_hire("Aria"), tenant_a, "human:a@acme.test")
        .await
        .expect("commit A's first hire");

    // A-sees-own-hire: A's `/api/v1/team` (list_workers(tenant_a)) returns it.
    let roster_a = identities
        .list_workers(tenant_a, false)
        .await
        .expect("A roster");
    assert_eq!(roster_a.len(), 1, "A must see their own hire");
    assert_eq!(roster_a[0].id, worker_id, "A's roster carries the hire");

    // B-does-not: a different user's tenant never sees A's hire.
    let roster_b = identities
        .list_workers(tenant_b, false)
        .await
        .expect("B roster");
    assert!(roster_b.is_empty(), "B must not see A's hire");

    // cwd-does-not: the shared cwd/demo tenant never carries A's hire.
    let roster_demo = identities
        .list_workers(demo_tenant, false)
        .await
        .expect("demo roster");
    assert!(
        roster_demo.is_empty(),
        "the cwd/demo tenant must not contain A's hire"
    );
}

fn read_src(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The migrated write sites resolve via the canonical resolver and no
/// longer cwd-resolve the write tenant. A revert to the cwd tenant
/// re-opens the broken loop (A hires → A's `/team` empty), so this pins
/// against that drift.
#[test]
fn migrated_write_sites_use_canonical_resolver_not_cwd() {
    // first-hire WRITE path.
    let onboarding = read_src("src/handlers/gateway/routes_onboarding.rs");
    assert!(
        onboarding.contains("resolve_tenant_for_request"),
        "routes_onboarding.rs first-hire must resolve the write tenant via \
         the canonical resolve_tenant_for_request — a cwd write would orphan \
         the hire from the JWT-aware /team read."
    );
    assert!(
        !onboarding.contains("cwd_write_tenant"),
        "routes_onboarding.rs first-hire must no longer call cwd_write_tenant."
    );

    // worker run-now WRITE path.
    let worker_run = read_src("src/handlers/gateway/routes_worker_run.rs");
    assert!(
        worker_run.contains("resolve_tenant_for_request"),
        "routes_worker_run.rs must resolve tenant via the canonical resolver."
    );
    assert!(
        !worker_run.contains("std::env::current_dir"),
        "routes_worker_run.rs must no longer cwd-resolve the tenant."
    );

    // the dead cwd shim is removed from the onboarding helpers.
    let helpers = read_src("src/handlers/gateway/routes_onboarding_helpers.rs");
    assert!(
        !helpers.contains("pub(super) async fn cwd_write_tenant"),
        "the cwd_write_tenant shim must be removed now that first-hire is \
         migrated to the canonical resolver."
    );
}
