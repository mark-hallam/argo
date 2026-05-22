//! Cross-tenant capability negative test for the macOS computer-use
//! executor (mission-cu-macos-cross-tenant-capability-test).
//!
//! **Ship status: FINDINGS-ONLY.** The brief asks for an
//! `#[ignore]`'d test plus a finding when the executor turns out to
//! have no tenant-aware capability gate. That is exactly what this
//! file does — the tests below document the contract the
//! conductor-authored fix-brief will need to deliver, and the
//! `#[ignore]` keeps them from running until that landing is on
//! main.
//!
//! ## The gap
//!
//! - `MacosCuRunRequest` carries only `action_id` + `system_prompt`
//!   + `audit_dir` + `anthropic_api_key`. **No `tenant_id`, no
//!   `worker_id`.** There is no place in the executor's run-time
//!   surface to bind a request to a tenant for a per-call
//!   capability check.
//! - `MacosCuExecutor::run`'s "pre-flight" is
//!   `crate::preflight::probe()` which checks the four macOS
//!   permissions (Screen Recording / Accessibility / Input
//!   Monitoring / Automation). **It is NOT a capability gate.**
//! - `MacosCuError` carries `NotMacos`, `PermissionDenied`,
//!   `AppNotAllowed`, `ActionNotAllowed`, `FrontAppMismatch`,
//!   subprocess + protocol variants — **no `CapabilityDenied` or
//!   `TenantMismatch` variant.**
//!
//! Mission claim 5 (real-world execution) requires the macOS
//! computer-use executor to refuse a tenant-mismatched
//! `(caller_tenant, worker_id)` at pre-flight before any screen
//! capture or `osascript` invocation runs. Until that lands, a
//! compromised tenant_a worker that obtains `worker_b`'s id could
//! drive tenant_b's computer-use session — provided some upstream
//! caller hands it to the executor. The SQL primitives are
//! tenant-scoped (verified by the positive sanity check below);
//! the per-call capability gate at the executor layer is the
//! missing piece.
//!
//! ## Removing `#[ignore]`
//!
//! Drop the attribute on both tests once the fix-brief delivers:
//!   1. `MacosCuRunRequest` grows `tenant_id: TenantId` and
//!      `worker_id: WorkerId` (or carries an opaque caller-
//!      capability token signed against tenant_id).
//!   2. `MacosCuExecutor::new` (or `run`) gains a handle to
//!      `IdentityStore` so the pre-flight can look up
//!      `worker.tenant_id`.
//!   3. Pre-flight refuses the call with a typed error variant —
//!      most likely `MacosCuError::CapabilityDenied { reason }` or
//!      `MacosCuError::TenantMismatch { caller, worker_tenant }`.
//!
//! Source: evaluator round-4 finding documented at
//! `docs/skycot/evals/scheduled/2026-05-22/computer-use-macos-executor-unit-tests.md`.

use argo_executor_computer_use_macos::{MacosCuConfig, MacosCuExecutor};
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;

/// Two-tenant bootstrap shared by both tests.
struct Bootstrap {
    store: SessionStore,
    tenant_a: TenantId,
    tenant_b: TenantId,
    worker_a: Worker,
    worker_b: Worker,
}

async fn bootstrap_two_tenants() -> Bootstrap {
    let store = SessionStore::open(":memory:").await.expect("open :memory:");
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.expect("ensure identity schema");
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let worker_a = Worker::draft(
        tenant_a,
        "test-vertical",
        Persona::named("alice", "test"),
        Role::new("tenant_a operator"),
    );
    let worker_b = Worker::draft(
        tenant_b,
        "test-vertical",
        Persona::named("bob", "test"),
        Role::new("tenant_b operator"),
    );
    identities.create_worker(&worker_a).await.expect("create worker_a");
    identities.create_worker(&worker_b).await.expect("create worker_b");
    Bootstrap {
        store,
        tenant_a,
        tenant_b,
        worker_a,
        worker_b,
    }
}

/// Sanity check that runs today (does NOT bear the `#[ignore]`):
/// the SQL primitives ARE tenant-scoped — `worker_b` is not
/// visible from `tenant_a`. The cross-tenant guard that's missing
/// is the executor-level capability gate, not the underlying
/// store query.
#[tokio::test]
async fn sql_primitives_are_tenant_scoped_today() {
    let setup = bootstrap_two_tenants().await;
    let identities = IdentityStore::new(&setup.store);
    let from_a = identities
        .get_worker(setup.tenant_a, setup.worker_b.id)
        .await
        .expect("lookup ok");
    assert!(
        from_a.is_none(),
        "worker_b must not be visible from tenant_a (SQL tenant scoping verified)"
    );
    let from_b = identities
        .get_worker(setup.tenant_b, setup.worker_b.id)
        .await
        .expect("lookup ok");
    assert!(
        from_b.is_some(),
        "worker_b is visible from its own tenant_b (positive sanity)"
    );
}

/// **Negative**: executor refuses a `(caller=tenant_a, worker_id=worker_b)`
/// pre-flight with a typed error. **`#[ignore]`'d** — the executor
/// has no surface to express the cross-tenant call today, so this
/// contract cannot be exercised. Drop the attribute when the
/// fix-brief lands.
#[tokio::test]
#[ignore = "FINDINGS-ONLY: MacosCuRunRequest has no tenant_id/worker_id; \
            MacosCuError has no CapabilityDenied/TenantMismatch variant. \
            Conductor: author a fix-brief to grow the executor's capability \
            pre-flight, then remove this #[ignore]."]
async fn cross_tenant_worker_id_is_refused_at_preflight() {
    let setup = bootstrap_two_tenants().await;
    let _executor = MacosCuExecutor::new(MacosCuConfig::for_notes_app())
        .expect("construct executor with default config");
    // The shape this test wants to express, once the executor is
    // tenant-aware:
    //
    //   let req = MacosCuRunRequest {
    //       tenant_id: setup.tenant_a,
    //       worker_id: setup.worker_b.id,
    //       action_id: "act-1".into(),
    //       /* ... other fields ... */
    //   };
    //   let outcome = executor.run(req).await;
    //   assert!(matches!(
    //       outcome,
    //       Err(MacosCuError::CapabilityDenied { .. })
    //         | Err(MacosCuError::TenantMismatch { .. })
    //   ));
    //
    // Today, `MacosCuRunRequest` carries no tenant/worker fields,
    // so the call cannot even be constructed. Silenced via
    // `#[ignore]` rather than `unimplemented!()` so the test
    // skeleton survives clippy / cargo test without misleading
    // CI into thinking the contract is enforced.
    let _ = (setup.tenant_a, setup.worker_b.id);
}

/// **Positive**: a same-tenant call passes the (would-be)
/// capability pre-flight gate. The actual `run()` may still fail
/// downstream (no Screen Recording grant in CI, missing
/// Anthropic API key, etc.) — the contract is about the gate, not
/// the action completing. **`#[ignore]`'d** for the same reason as
/// the negative test.
#[tokio::test]
#[ignore = "FINDINGS-ONLY: pairs with `cross_tenant_worker_id_is_refused_at_preflight`. \
            Remove together once the executor grows tenant-aware MacosCuRunRequest."]
async fn same_tenant_worker_id_passes_preflight_gate() {
    let setup = bootstrap_two_tenants().await;
    let _executor = MacosCuExecutor::new(MacosCuConfig::for_notes_app())
        .expect("construct executor with default config");
    // The shape this test wants to express:
    //
    //   let req = MacosCuRunRequest {
    //       tenant_id: setup.tenant_a,
    //       worker_id: setup.worker_a.id,
    //       action_id: "act-1".into(),
    //       /* ... */
    //   };
    //   let outcome = executor.run(req).await;
    //   // Pre-flight should NOT refuse with CapabilityDenied /
    //   // TenantMismatch. Any other variant (PermissionDenied,
    //   // NotMacos, etc.) is acceptable — that's a downstream
    //   // gate, not the capability one this test asserts.
    //   assert!(!matches!(
    //       outcome,
    //       Err(MacosCuError::CapabilityDenied { .. })
    //         | Err(MacosCuError::TenantMismatch { .. })
    //   ));
    let _ = (setup.tenant_a, setup.worker_a.id);
}
