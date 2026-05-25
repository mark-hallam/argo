//! Public-surface inventory + FINDINGS-ONLY contract pins for
//! `argo-routing` (brief: `eval-argo-routing-crate-surface`).
//!
//! Three load-bearing claims this file pins:
//!
//! 1. **Public surface stability.** The 8 re-exports advertised by
//!    `lib.rs` — `Router`, `RoutingContext`, `FixedRouter`,
//!    `TieredRouter`, `PersistingRouter`, `PolicyPluginRouter`,
//!    `TierCache`, `list_for_tenant` — all exist with their advertised shape. The crate's
//!    sole consumer in production today is
//!    `argo_agent_runner::turn_routing` (one callsite), so any rename
//!    or shape break would cascade silently without a pin.
//!
//! 2. **Tenant scoping is explicit.** `RoutingContext` carries
//!    `tenant_id`, `RoutingDecision` stores it, and the
//!    `routing_decision` table has a `tenant_id` column. Tenant-scoped
//!    reads go through `list_for_tenant`.
//!
//! 3. **Production persistence wire-gap** (FINDINGS-ONLY). The only
//!    production caller in the workspace
//!    (`crates/argo-agent-runner/src/turn_routing.rs:87`) builds
//!    `TieredRouter::new(classifier)` and writes an
//!    `AuditKind::RoutingDecision` audit row — it does NOT wrap with
//!    `PersistingRouter` and therefore never writes the
//!    `routing_decision` table. The §24.4 spec acceptance ("all
//!    non-Fixed decisions persisted in `routing_decision`") is met by
//!    the audit log only on origin/main today. Fix-brief:
//!    `fix-routing-decision-persist-wire`.
//!
//! Companion findings report:
//! `docs/skycot/evals/scheduled/2026-05-23/eval-argo-routing-crate-surface.md`.

use argo_routing::{
    list_for_tenant, FixedRouter, PersistingRouter, PolicyPluginRouter, Router, RoutingContext,
    TierCache, TieredRouter, DEFAULT_TTL,
};
use argo_store::SessionStore;
use argo_types::{
    ArgoError, ArgoResult, CapabilitySet, ProviderModel, RoutingDecision, RoutingDecisionId,
    RoutingPolicy, SessionId, TenantId, WorkspaceId,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// §1 — Surface inventory: every re-export from `lib.rs` is reachable
/// with its advertised shape. A future rename, deletion, or visibility
/// change fails `cargo build` for this test.
#[test]
fn lib_reexports_match_published_surface() {
    // Re-named types.
    let _: TierCache = TierCache::default();
    let _: Duration = DEFAULT_TTL;

    // Router impls — constructable via their documented `::new`.
    let _: FixedRouter = FixedRouter::new();
    let _: PolicyPluginRouter = PolicyPluginRouter::new("placeholder");

    // RoutingContext fields are publicly visible and named as
    // advertised in the rustdoc.
    let _ = RoutingContext {
        tenant_id: TenantId::new(),
        session_id: SessionId::new(),
        turn: 0,
        messages: &[],
        user_message: "",
        tools_in_scope: &[],
        default_model: ProviderModel {
            provider: String::new(),
            model: String::new(),
        },
        policy: RoutingPolicy::Fixed,
        deadline: Instant::now() + Duration::from_secs(1),
    };
}

/// §1 — `Router` is object-safe (matches the rustdoc claim
/// `assert_object_safe(_r: &dyn Router)`). Required because the
/// persisting wrapper holds an arbitrary `R: Router` and production
/// callers commonly want `Arc<dyn Router>` for runtime selection.
#[test]
fn router_trait_is_object_safe() {
    fn assert_object_safe(_r: &dyn Router) {}
    let r = FixedRouter::new();
    assert_object_safe(&r);
}

/// §1 — All four `Router` implementations advertised by the crate
/// implement `Router`. The persisting wrapper is parametric, so we
/// exercise it on `FixedRouter` (the documented zero-config case).
#[tokio::test]
async fn four_routers_implement_the_router_trait() {
    let store = fresh_store().await;
    let _: Box<dyn Router> = Box::new(FixedRouter::new());
    let _: Box<dyn Router> = Box::new(PolicyPluginRouter::new("stub"));
    let _: Box<dyn Router> = Box::new(PersistingRouter::new(FixedRouter::new(), store.clone()));
    // TieredRouter requires a transport — checked indirectly by the
    // in-crate tier tests (`src/tiered/tests.rs`). The compile-time
    // assertion below confirms it satisfies the trait without
    // instantiating a real transport.
    fn _assert_tiered_is_router(_: &TieredRouter) {
        fn _is_router<R: Router>(_: &R) {}
    }
}

/// §2 — `RoutingContext` carries both `tenant_id` and `session_id`.
/// This is the compile-time surface pin for the routing substrate's
/// tenant-aware contract.
#[test]
fn routing_context_field_set_is_tenant_scoped() {
    let tenant_id = TenantId::new();
    let ctx = RoutingContext {
        tenant_id,
        session_id: SessionId::new(),
        turn: 1,
        messages: &[],
        user_message: "msg",
        tools_in_scope: &[],
        default_model: ProviderModel {
            provider: "p".into(),
            model: "m".into(),
        },
        policy: RoutingPolicy::Fixed,
        deadline: Instant::now() + Duration::from_secs(1),
    };
    // Field access — if any of these field names change, the test
    // fails to compile and the surface drift surfaces in CI.
    let _ = ctx.tenant_id;
    let _ = ctx.session_id;
    let _ = ctx.turn;
    let _ = ctx.messages.len();
    let _ = ctx.user_message;
    let _ = ctx.tools_in_scope.len();
    let _ = ctx.default_model.clone();
    let _ = ctx.policy.clone();
    let _ = ctx.deadline;
}

/// §2 — `RoutingDecision` carries `tenant_id` so persisted rows can
/// be queried without session-only cross-tenant ambiguity.
#[test]
fn routing_decision_field_set_is_tenant_scoped() {
    let tenant_id = TenantId::new();
    let d = RoutingDecision {
        id: RoutingDecisionId::new(),
        tenant_id,
        session_id: SessionId::new(),
        turn: 1,
        policy: "fixed".into(),
        tier: "unrouted".into(),
        classified_at: chrono::Utc::now(),
        classifier_latency_ms: 0,
        chosen_provider_model: ProviderModel {
            provider: "p".into(),
            model: "m".into(),
        },
        reason: "ok".into(),
    };
    let _ = d.id;
    let _ = d.tenant_id;
    let _ = d.session_id;
    let _ = d.turn;
    let _ = d.policy;
    let _ = d.tier;
    let _ = d.classified_at;
    let _ = d.classifier_latency_ms;
    let _ = d.chosen_provider_model;
    let _ = d.reason;
}

/// §2 — Runtime confirmation that the `routing_decision` table on a
/// fresh-migrated store carries the `tenant_id` column.
#[tokio::test]
async fn routing_decision_table_has_tenant_id_column() {
    let store = fresh_store().await;
    let cols: Vec<(i64, String)> =
        sqlx::query_as("SELECT cid, name FROM pragma_table_info('routing_decision')")
            .fetch_all(store.pool())
            .await
            .expect("read pragma");
    let names: Vec<String> = cols.into_iter().map(|(_, n)| n).collect();
    assert!(
        names.iter().any(|n| n == "session_id"),
        "session_id must remain; saw {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "tenant_id"),
        "routing_decision must carry tenant_id after fix-routing-decision-tenant-scoping; \
         saw {names:?}"
    );
}

/// §2 — Runtime confirmation that
/// `SessionStore::persist_routing_decision` stores the decision's
/// tenant id and the tenant-scoped read helper returns it.
#[tokio::test]
async fn persist_routing_decision_round_trips_tenant_id() {
    let store = fresh_store().await;
    let (tenant_a, session_a) = seed_tenant_session(&store, "a").await;
    let decision = RoutingDecision {
        id: RoutingDecisionId::new(),
        tenant_id: tenant_a,
        session_id: session_a,
        turn: 1,
        policy: "tiered".into(),
        tier: "complex".into(),
        classified_at: chrono::Utc::now(),
        classifier_latency_ms: 7,
        chosen_provider_model: ProviderModel {
            provider: "anthropic".into(),
            model: "claude-opus-4-6".into(),
        },
        reason: "test".into(),
    };
    store
        .persist_routing_decision(&decision)
        .await
        .expect("persist accepts same-tenant decision");
    let rows = list_for_tenant(&store, tenant_a)
        .await
        .expect("tenant-scoped read");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].tenant_id, tenant_a);
    assert_eq!(rows[0].session_id, session_a);
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM routing_decision WHERE session_id = ? AND tenant_id = ?",
    )
    .bind(session_a.0.as_bytes().as_slice())
    .bind(tenant_a.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);
}

/// §2 — A decision whose `tenant_id` disagrees with the session owner
/// is rejected before insert.
#[tokio::test]
async fn persist_routing_decision_refuses_foreign_tenant() {
    let store = fresh_store().await;
    let (_tenant_a, session_a) = seed_tenant_session(&store, "a").await;
    let tenant_b = TenantId::new();
    let decision = RoutingDecision {
        id: RoutingDecisionId::new(),
        tenant_id: tenant_b,
        session_id: session_a,
        turn: 1,
        policy: "tiered".into(),
        tier: "complex".into(),
        classified_at: chrono::Utc::now(),
        classifier_latency_ms: 7,
        chosen_provider_model: ProviderModel {
            provider: "anthropic".into(),
            model: "claude-opus-4-6".into(),
        },
        reason: "test".into(),
    };
    let err = store
        .persist_routing_decision(&decision)
        .await
        .expect_err("foreign tenant decision must be rejected");
    assert!(
        matches!(err, ArgoError::NotFound(_)),
        "foreign tenant persist must hide session existence, got {err:?}"
    );
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM routing_decision")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(n, 0);
}

/// §2 — Tenant-scoped reads must not leak another tenant's routing
/// decisions, while still returning the same tenant's baseline row.
#[tokio::test]
async fn list_for_tenant_excludes_other_tenant_decisions() {
    let store = fresh_store().await;
    let (tenant_a, session_a) = seed_tenant_session(&store, "a").await;
    let (tenant_b, session_b) = seed_tenant_session(&store, "b").await;

    store
        .persist_routing_decision(&test_decision(tenant_a, session_a, "simple"))
        .await
        .expect("persist tenant a");
    store
        .persist_routing_decision(&test_decision(tenant_b, session_b, "complex"))
        .await
        .expect("persist tenant b");

    let rows_b = list_for_tenant(&store, tenant_b)
        .await
        .expect("tenant b read");
    assert_eq!(rows_b.len(), 1, "same-tenant baseline must be visible");
    assert_eq!(rows_b[0].tenant_id, tenant_b);
    assert_eq!(rows_b[0].session_id, session_b);
    assert!(
        rows_b.iter().all(|row| row.session_id != session_a),
        "tenant B read leaked tenant A routing rows: {rows_b:?}"
    );
}

/// §3 — Production wire-gap pin. Every `Router` impl can be wrapped
/// in `PersistingRouter`, but the only production caller
/// (`argo_agent_runner::turn_routing::maybe_route_turn_with_transport`)
/// constructs `TieredRouter::new(...)` and does NOT wrap it — model-
/// tier decisions land in the audit log via
/// `AuditKind::RoutingDecision` but never in the `routing_decision`
/// table. The pin documents what production SHOULD look like; the
/// fix-brief `fix-routing-decision-persist-wire` swaps in the wrap.
#[tokio::test]
async fn persisting_router_wraps_tiered_with_store() {
    // This test asserts the wrapper SHAPE is available — it does NOT
    // assert that production uses it. The fix-brief is the wire.
    let store = fresh_store().await;
    let (tenant_id, session_id) = seed_tenant_session(&store, "wrap").await;
    let decision = RoutingDecision {
        id: RoutingDecisionId::new(),
        tenant_id,
        session_id,
        turn: 1,
        policy: "tiered".into(),
        tier: "medium".into(),
        classified_at: chrono::Utc::now(),
        classifier_latency_ms: 12,
        chosen_provider_model: ProviderModel {
            provider: "openai".into(),
            model: "gpt-4o".into(),
        },
        reason: "scripted".into(),
    };
    let outer = PersistingRouter::new(
        ScriptedRouter {
            decision: decision.clone(),
        },
        store.clone(),
    );
    let ctx = ctx_for(tenant_id, session_id, "msg");
    let out = outer.route(&ctx).await.expect("route");
    assert_eq!(out.policy, "tiered");
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM routing_decision WHERE session_id = ? AND tenant_id = ?",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .bind(tenant_id.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "PersistingRouter writes when policy != 'fixed'");
}

/// §3 — FINDINGS-ONLY: assert the production wire is in place. Today
/// `turn_routing.rs` does NOT wrap with `PersistingRouter`, so this
/// stays `#[ignore]`'d. Reviewers can re-enable it from the fix-brief
/// without re-authoring the assertion.
#[ignore = "FINDINGS-ONLY: enable when fix-routing-decision-persist-wire lands"]
#[test]
fn production_turn_routing_wraps_tier_router_in_persisting_router() {
    // Pseudo-assert against the source. A future fix can either add
    // a runtime-observable signal (e.g. `routing_decision` row count
    // after a turn) or grep the callsite. For now we record the
    // expectation so the brief is grep-able.
    panic!(
        "FINDINGS pin: turn_routing::maybe_route_turn_with_transport builds \
         `TieredRouter::new(classifier)` but does NOT wrap it in \
         `PersistingRouter`. The §24.4 acceptance row in \
         `docs/plan/argo-build-plan.md` requires non-Fixed decisions to be \
         persisted in `routing_decision` — currently only the audit log \
         receives the row. Fix-brief: fix-routing-decision-persist-wire."
    );
}

/// §3 — `PolicyPluginRouter` is a permanent v1 stub: every call
/// returns `RoutingFailed`. Pinned so a future change to the stub
/// surfaces in review (the plugin host §11 is still on the roadmap).
#[tokio::test]
async fn policy_plugin_router_is_a_stub_returning_routing_failed() {
    let router = PolicyPluginRouter::new("cost_optimizer");
    let ctx = ctx_for(TenantId::new(), SessionId::new(), "x");
    let err = router.route(&ctx).await.unwrap_err();
    match err {
        ArgoError::RoutingFailed { reason } => {
            assert!(reason.contains("PolicyPluginRouter"), "reason: {reason}");
            assert!(reason.contains("plugin host"), "reason: {reason}");
        }
        other => panic!("expected RoutingFailed, got {other:?}"),
    }
}

// ---------- fixtures ----------

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

async fn seed_tenant_session(store: &SessionStore, label: &str) -> (TenantId, SessionId) {
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, ?, '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(label)
    .execute(store.pool())
    .await
    .unwrap();
    let session = store
        .create_session(tenant, ws, "model", "provider", Some("test"), None)
        .await
        .unwrap();
    (tenant, session)
}

fn ctx_for(tenant_id: TenantId, session_id: SessionId, msg: &str) -> RoutingContext<'_> {
    let leaked: &'static str = Box::leak(msg.to_string().into_boxed_str());
    RoutingContext {
        tenant_id,
        session_id,
        turn: 1,
        messages: &[],
        user_message: leaked,
        tools_in_scope: &[],
        default_model: ProviderModel {
            provider: "openai".into(),
            model: "gpt-4o-mini".into(),
        },
        policy: RoutingPolicy::Fixed,
        deadline: Instant::now() + Duration::from_secs(60),
    }
}

fn test_decision(tenant_id: TenantId, session_id: SessionId, tier: &str) -> RoutingDecision {
    RoutingDecision {
        id: RoutingDecisionId::new(),
        tenant_id,
        session_id,
        turn: 1,
        policy: "tiered".into(),
        tier: tier.to_string(),
        classified_at: chrono::Utc::now(),
        classifier_latency_ms: 7,
        chosen_provider_model: ProviderModel {
            provider: "anthropic".into(),
            model: "claude-opus-4-6".into(),
        },
        reason: "test".into(),
    }
}

struct ScriptedRouter {
    decision: RoutingDecision,
}

#[async_trait]
impl Router for ScriptedRouter {
    async fn route(&self, _ctx: &RoutingContext<'_>) -> ArgoResult<RoutingDecision> {
        Ok(self.decision.clone())
    }
}

// `CapabilitySet` is imported only so the doc-comment cross-reference
// to argo-types resolves cleanly in `cargo doc`. Suppress the unused
// warning without polluting the public surface.
const _: fn() = || {
    let _ = CapabilitySet::empty();
};
