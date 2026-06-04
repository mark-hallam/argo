//! Cross-tenant isolation regression test for sprint brief
//! `sprint-ws1-4-cross-tenant-isolation-regression-test`.
//!
//! ## What the keystone guarantees, and the gap this closes
//!
//! The keystone (#2676) made every signed-in user's read routes resolve to
//! the user-bound tenant carried in their `__Host-session` JWT, not the
//! gateway process's cwd. The resolver itself is exhaustively unit-tested
//! crate-internal in `src/handlers/gateway/resolve_tenant_tests.rs`:
//! `jwt_session_resolves_to_user_tenant_not_cwd_or_other_user`,
//! `jwt_session_wins_over_explicit_header` (the `X-Argo-Tenant-Id`-override
//! case — review follow-up #4), and the cwd/anonymous fallback. The sibling
//! `tenant_resolve_from_session_jwt.rs` adds the data-isolation proof for a
//! *fresh* user (empty roster) plus structural pins.
//!
//! What none of them proved is the **populated** case across all six
//! migrated surfaces: when tenants A AND B each own rich, distinct data,
//! the exact tenant-scoped read each migrated route performs returns ONLY
//! the owner's rows — never the other tenant's, never the cwd/demo
//! tenant's. This brief locks that property so a future change that
//! re-pools reads onto one tenant fails CI loudly.
//!
//! ## Why this is an in-process store-read assertion, not a gateway-boot
//! HTTP probe (sensible deviation — see PR body)
//!
//! The brief sketched a gateway-boot HTTP test. That was implemented and
//! abandoned: gateway-boot integration tests are a documented intermittent
//! `GatewayCrashed` flake on this build machine (see the rationale in
//! `gateway_uuid_routes_tenant_ownership.rs` and `tenant_resolve_from_
//! session_jwt.rs`, which both chose structural assertions for exactly
//! this reason). It is RED *today* on current `origin/main`: the existing
//! `gateway_health_smoke` boots a gateway and crashes within ~6s under the
//! build swarm's memory pressure (macOS jetsam — the same pressure
//! documented in `CLAUDE.md`). A regression test must be reliable, so this
//! one proves the SAME property the HTTP path would, one layer down: it
//! drives the **exact tenant-scoped store reads each migrated route
//! calls**, having seeded three real tenants (A, B, and a cwd/demo
//! stand-in) with distinct data through the real per-tenant write paths.
//! Composed with the resolver unit tests (JWT→tenant + override-wins) and
//! the structural pins below (route → `resolve_tenant_for_request`, no cwd
//! copy), a route reverting to `resolve_workspace_tenant` is caught: the
//! pin goes red, and the populated-isolation assertions document the data
//! contract the pin protects.
//!
//! ## Route → store-read mapping asserted here
//!
//! | Route                          | Tenant-scoped read the route performs            |
//! |--------------------------------|--------------------------------------------------|
//! | `GET /api/v1/team`             | `IdentityStore::list_workers(tenant, false)`     |
//! | `GET /api/v1/workers/<id>`     | `IdentityStore::get_worker(tenant, id)`          |
//! | `GET /api/v1/workers/<id>/overview` | `IdentityStore::get_worker(tenant, id)`     |
//! | `GET /skills/<name>/usage`     | `IdentityStore::workers_with_skill_learned_at`   |
//! | `GET /api/v1/tenant/usage`     | `tenant_query::tenant_usage_for_period`          |
//! | `GET /api/v1/audit`            | `audit_record WHERE tenant_id = <resolved>`      |
//! | `GET /api/v1/approvals`        | `approvals::list_pending(tenant)` (seeded arm)   |

use argo_audit::{AuditKind, AuditSink};
use argo_identity::user::UserAccount;
use argo_identity::user_store::insert_user;
use argo_identity::{
    provision_for_user, IdentityStore, Persona, ProvisionRequest, Role, Worker, WorkerId,
};
use argo_skill::SkillStore;
use argo_store::approvals::{create_request, list_pending, DefaultDecision, NewApprovalRequest};
use argo_store::tenant_query::{increment_tenant_usage, tenant_usage_for_period};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;

// ───────────────────────── fixtures ─────────────────────────

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// Provision a fresh user → returns the user-bound tenant (the JWT path).
async fn provision(store: &Arc<SessionStore>, email: &str, ws: &str) -> TenantId {
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

/// Everything one tenant owns, named so a leak is unambiguous.
struct Seeded {
    tenant: TenantId,
    worker: WorkerId,
    worker_name: String,
    audit_probe: String,
    approval_marker: String,
    usage: u64,
}

/// Seed one worker + audit row + approval + usage into `tenant`, plus have
/// its worker learn `skill_id` (so `/skills/<name>/usage` is exercised).
async fn seed(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    tag: &str,
    usage: u64,
    skill_id: &str,
) -> Seeded {
    let worker_name = format!("Worker-{tag}");
    let worker = Worker::draft(
        tenant,
        "isolation-regression",
        Persona::named(worker_name.clone(), "iso@regression.test"),
        Role::new("Cross-tenant isolation fixture worker"),
    );
    let worker_id = worker.id;
    let identity = IdentityStore::new(store.as_ref());
    identity.create_worker(&worker).await.expect("create worker");
    identity
        .add_worker_skill(worker_id, skill_id, 3)
        .await
        .expect("learn skill");

    let audit_probe = format!("audit-probe-{tag}");
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    sink.write(
        None,
        AuditKind::HttpRequest,
        serde_json::json!({ "probe": audit_probe }),
        None,
    )
    .await
    .expect("audit write");
    sink.flush_pending().await.expect("audit flush");

    let approval_marker = format!("approval-{tag}");
    create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            tenant_id: tenant,
            requested_by: &approval_marker,
            deadline_at: Utc::now().timestamp() + 3600,
            default_decision: DefaultDecision::Ask,
            draft_content: Some(&approval_marker),
        },
    )
    .await
    .expect("create approval_request");

    increment_tenant_usage(store, tenant, usage, Utc::now())
        .await
        .expect("usage");

    Seeded {
        tenant,
        worker: worker_id,
        worker_name,
        audit_probe,
        approval_marker,
        usage,
    }
}

/// Count `tenant`'s audit rows whose payload carries `probe`. Mirrors the
/// `/api/v1/audit` default-scope read (`audit_record WHERE tenant_id = ?`).
async fn audit_probe_count(store: &Arc<SessionStore>, tenant: TenantId, probe: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE tenant_id = ? AND payload_json LIKE ?",
    )
    .bind(tenant.to_string())
    .bind(format!("%{probe}%"))
    .fetch_one(store.pool())
    .await
    .expect("audit count")
}

// ───────────────────── the populated-isolation test ─────────────────────

/// Three real tenants (A, B, and a cwd/demo stand-in), each seeded with
/// distinct data, prove that the tenant-scoped read behind every migrated
/// route returns ONLY the owner's rows — never another tenant's, never the
/// cwd/demo tenant's — symmetrically. This is the populated counterpart to
/// the keystone's fresh-user empty-roster proof.
#[tokio::test]
async fn populated_tenants_isolate_across_six_migrated_route_reads() {
    let store = fresh_store().await;

    let tenant_a = provision(&store, "a@acme.test", "Acme Workspace").await;
    let tenant_b = provision(&store, "b@beta.test", "Beta Workspace").await;
    // The cwd/demo tenant the OLD per-file `resolve_workspace_tenant`
    // collapsed every signed-in read onto. Seeded so a regression that
    // re-pools onto it would surface its rows under A or B.
    let (tenant_demo, _ws) = store
        .ensure_workspace("demo-workspace", "/skycot/demo-cwd")
        .await
        .expect("ensure demo workspace");
    assert_ne!(tenant_a, tenant_b);
    assert_ne!(tenant_a, tenant_demo);
    assert_ne!(tenant_b, tenant_demo);

    // One GLOBAL skill, learned by every tenant's worker — proves
    // `/skills/<name>/usage` `workers_using` is tenant-filtered, not the
    // global adoption set.
    let skill_raw = "---\nname: iso-shared-skill\nversion: \"0.1.0\"\ndescription: \"iso\"\nprovenance:\n  source: builtin\n  hash: sha256-iso-shared\n---\nbody\n";
    let skill = SkillStore::new(Arc::clone(&store))
        .install(skill_raw, None)
        .await
        .expect("install skill");
    let skill_id = skill.id.0.to_string();

    let a = seed(&store, tenant_a, "ALPHA", 7, &skill_id).await;
    let b = seed(&store, tenant_b, "BETA", 3, &skill_id).await;
    let demo = seed(&store, tenant_demo, "DEMO", 99, &skill_id).await;

    let identity = IdentityStore::new(store.as_ref());

    // For every tenant, assert its read sees ONLY its own marker and never
    // the other two — across all six route surfaces.
    for (me, others) in [(&a, [&b, &demo]), (&b, [&a, &demo]), (&demo, [&a, &b])] {
        // ── /api/v1/team → list_workers(tenant, false)
        let roster = identity
            .list_workers(me.tenant, false)
            .await
            .expect("list_workers");
        let names: Vec<&str> = roster.iter().map(|w| w.persona.name.as_str()).collect();
        assert!(
            names.contains(&me.worker_name.as_str()),
            "team read for {} missing its own worker — a cwd regression empties \
             signed-in rosters: {names:?}",
            me.worker_name
        );
        for other in others {
            assert!(
                !names.contains(&other.worker_name.as_str()),
                "team read for {} leaked {}'s worker: {names:?}",
                me.worker_name,
                other.worker_name
            );
        }

        // ── /api/v1/workers/<id> + /overview → get_worker(tenant, id)
        assert!(
            identity
                .get_worker(me.tenant, me.worker)
                .await
                .expect("get_worker own")
                .is_some(),
            "worker-detail read for {} cannot see its OWN worker",
            me.worker_name
        );
        for other in others {
            assert!(
                identity
                    .get_worker(me.tenant, other.worker)
                    .await
                    .expect("get_worker cross")
                    .is_none(),
                "worker-detail read for {} resolved {}'s worker id cross-tenant \
                 (must be hidden)",
                me.worker_name,
                other.worker_name
            );
        }

        // ── /skills/<name>/usage → workers_with_skill_learned_at(tenant, skill)
        let users = identity
            .workers_with_skill_learned_at(me.tenant, &skill_id)
            .await
            .expect("workers_with_skill");
        let user_names: Vec<&str> = users.iter().map(|(w, _)| w.persona.name.as_str()).collect();
        assert_eq!(
            user_names,
            vec![me.worker_name.as_str()],
            "skill-usage read for {} must list only its own worker in workers_using",
            me.worker_name
        );

        // ── /api/v1/tenant/usage → tenant_usage_for_period(tenant)
        let (used, _) = tenant_usage_for_period(&store, me.tenant, Utc::now())
            .await
            .expect("usage read");
        assert_eq!(
            used, me.usage,
            "usage read for {} must report its OWN action count, not another tenant's",
            me.worker_name
        );

        // ── /api/v1/audit → audit_record WHERE tenant_id = resolved
        assert!(
            audit_probe_count(&store, me.tenant, &me.audit_probe).await >= 1,
            "audit read for {} cannot see its own probe",
            me.worker_name
        );
        for other in others {
            assert_eq!(
                audit_probe_count(&store, me.tenant, &other.audit_probe).await,
                0,
                "audit read for {} leaked {}'s audit probe",
                me.worker_name,
                other.worker_name
            );
        }

        // ── /api/v1/approvals → approvals::list_pending(tenant) (seeded arm)
        let pending = list_pending(store.pool(), me.tenant, 100)
            .await
            .expect("list_pending");
        let markers: Vec<&str> = pending.iter().map(|r| r.requested_by.as_str()).collect();
        assert!(
            markers.contains(&me.approval_marker.as_str()),
            "approvals read for {} cannot see its own approval: {markers:?}",
            me.worker_name
        );
        for other in others {
            assert!(
                !markers.contains(&other.approval_marker.as_str()),
                "approvals read for {} leaked {}'s approval: {markers:?}",
                me.worker_name,
                other.approval_marker
            );
        }
    }
}

// ───────────────────── structural regression pins ─────────────────────

fn read_src(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The regression-proof: every migrated read-route file routes tenant
/// resolution through the canonical JWT-aware `resolve_tenant_for_request`
/// (directly, or via the seeded-approvals `resolve_tenant` helper that
/// wraps it). Reverting any one of them to a private cwd
/// `resolve_workspace_tenant` copy re-opens the keystone bug and fails this
/// test — the acceptance-criterion revert experiment (documented in the PR
/// body) was validated against this assertion.
#[test]
fn migrated_routes_resolve_via_jwt_resolver_not_cwd() {
    // (file, token proving it flows through the canonical resolver)
    let routed = [
        ("src/handlers/gateway/routes_team_list.rs", "resolve_tenant_for_request"),
        ("src/handlers/gateway/routes_worker_overview.rs", "resolve_tenant_for_request"),
        ("src/handlers/gateway/routes_worker_detail.rs", "resolve_tenant_for_request"),
        ("src/handlers/gateway/routes_skills.rs", "resolve_tenant_for_request"),
        ("src/handlers/gateway/usage_endpoint.rs", "resolve_tenant_for_request"),
        ("src/handlers/gateway/routes_approvals_json_seeded.rs", "resolve_tenant"),
        // audit feed's default scope flows through the resolver wrapper.
        ("src/handlers/gateway/audit_query.rs", "resolve_tenant_for_request"),
    ];
    for (rel, token) in routed {
        let src = read_src(rel);
        assert!(
            src.contains(token),
            "{rel} no longer references `{token}` — the WS1 JWT-tenant \
             resolution regressed back toward the cwd tenant."
        );
    }

    // The migrated route files must NOT carry a private cwd
    // `resolve_workspace_tenant` definition or read `current_dir` for
    // tenant resolution — that was the keystone bug.
    let no_cwd_copy = [
        "src/handlers/gateway/routes_team_list.rs",
        "src/handlers/gateway/routes_worker_overview.rs",
        "src/handlers/gateway/routes_worker_detail.rs",
        "src/handlers/gateway/routes_skills.rs",
        "src/handlers/gateway/usage_endpoint.rs",
    ];
    for rel in no_cwd_copy {
        let src = read_src(rel);
        assert!(
            !src.contains("fn resolve_workspace_tenant"),
            "{rel} re-introduced a private cwd `resolve_workspace_tenant` copy."
        );
        assert!(
            !src.contains("std::env::current_dir"),
            "{rel} reads std::env::current_dir for tenant resolution — the cwd-tenant bug."
        );
    }
}
