//! Cross-tenant capability negative test for the macOS computer-use
//! executor (mission-cu-macos-executor-tenant-preflight-enforce).
//!
//! Originally shipped as FINDINGS-ONLY by
//! `mission-cu-macos-cross-tenant-capability-test` (#2063) — the
//! contract tests were `#[ignore]`'d because the executor had no
//! tenant-aware pre-flight surface to assert against. This
//! fix-pair brief delivers the surface; the `#[ignore]` attributes
//! are now dropped and the contract runs.
//!
//! ## What this asserts
//!
//! 1. `sql_primitives_are_tenant_scoped_today` — the SQL layer is
//!    tenant-scoped (sanity check, ran on FINDINGS-ONLY ship too).
//! 2. `cross_tenant_worker_id_returns_tenant_mismatch` — the
//!    executor refuses a request whose `(tenant_id, worker_id)`
//!    pair does not exist in that tenant, returning
//!    `MacosCuError::TenantMismatch` BEFORE any macOS API call
//!    runs. This is the negative-path contract for mission
//!    claim 5 (real-world execution).
//! 3. `same_tenant_worker_id_passes_pre_flight` — the positive
//!    pair: a request whose `(tenant_id, worker_id)` pair DOES
//!    exist passes the pre-flight gate. The run will still bail
//!    on the macOS-permission probe in CI / non-mac hosts, but
//!    the bail is at the macOS layer, NOT a `TenantMismatch`.

use argo_executor_computer_use_macos::{
    MacosCuConfig, MacosCuError, MacosCuExecutor, MacosCuRunRequest,
};
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::PathBuf;
use std::sync::Arc;

/// Two-tenant bootstrap shared by the cross-tenant tests.
struct Bootstrap {
    store: Arc<SessionStore>,
    tenant_a: TenantId,
    tenant_b: TenantId,
    worker_a: Worker,
    worker_b: Worker,
}

async fn bootstrap_two_tenants() -> Bootstrap {
    let store = SessionStore::open(":memory:").await.expect("open :memory:");
    let identities = IdentityStore::new(&store);
    identities
        .ensure_schema()
        .await
        .expect("ensure identity schema");
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
    identities
        .create_worker(&worker_a)
        .await
        .expect("create worker_a");
    identities
        .create_worker(&worker_b)
        .await
        .expect("create worker_b");
    Bootstrap {
        store: Arc::new(store),
        tenant_a,
        tenant_b,
        worker_a,
        worker_b,
    }
}

fn run_request(tenant_id: TenantId, worker_id: argo_identity::WorkerId) -> MacosCuRunRequest {
    MacosCuRunRequest {
        tenant_id,
        worker_id,
        action_id: "act-cross-tenant".into(),
        system_prompt: "noop".into(),
        audit_dir: PathBuf::from("/tmp/cross-tenant-audit-never-written"),
        anthropic_api_key: "test-key-never-used".into(),
    }
}

/// Sanity check: the SQL layer is tenant-scoped — `worker_b` is
/// not visible from `tenant_a`. (This ran on the FINDINGS-ONLY
/// PR too and still runs here.)
#[tokio::test]
async fn sql_primitives_are_tenant_scoped_today() {
    let setup = bootstrap_two_tenants().await;
    let identities = IdentityStore::new(setup.store.as_ref());
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

/// **Negative**: the executor refuses a
/// `(caller=tenant_a, worker_id=worker_b)` pre-flight with
/// `MacosCuError::TenantMismatch`. This is the contract enforced
/// BEFORE any macOS API call runs.
#[tokio::test]
async fn cross_tenant_worker_id_returns_tenant_mismatch() {
    let setup = bootstrap_two_tenants().await;
    let lookup: Arc<dyn argo_executor_computer_use_macos::TenantWorkerLookup> =
        Arc::clone(&setup.store) as _;
    let executor = MacosCuExecutor::new(MacosCuConfig::for_notes_app(), lookup)
        .expect("construct executor with default config + lookup");

    let req = run_request(setup.tenant_a, setup.worker_b.id);
    let err = executor
        .pre_flight_tenant_check(&req)
        .await
        .expect_err("cross-tenant pair must be refused at pre-flight");

    match err {
        MacosCuError::TenantMismatch {
            tenant_id,
            worker_id,
        } => {
            assert_eq!(tenant_id, setup.tenant_a);
            assert_eq!(worker_id, setup.worker_b.id);
        }
        other => panic!("expected TenantMismatch, got: {other:?}"),
    }
}

/// **Negative (absent worker)**: the executor refuses a
/// `(caller=tenant_a, worker_id=<fresh, never created>)` with
/// the same `TenantMismatch` variant. Same message shape as the
/// cross-tenant case so a caller cannot distinguish "worker does
/// not exist" from "worker is in another tenant" (existence-leak
/// invariant, matches
/// `mission-cross-tenant-worker-handler-no-existence-leak`).
#[tokio::test]
async fn absent_worker_id_returns_tenant_mismatch_shape() {
    let setup = bootstrap_two_tenants().await;
    let lookup: Arc<dyn argo_executor_computer_use_macos::TenantWorkerLookup> =
        Arc::clone(&setup.store) as _;
    let executor = MacosCuExecutor::new(MacosCuConfig::for_notes_app(), lookup)
        .expect("construct executor with default config + lookup");

    let absent_worker = argo_identity::WorkerId::new();
    let req = run_request(setup.tenant_a, absent_worker);
    let err = executor
        .pre_flight_tenant_check(&req)
        .await
        .expect_err("absent worker must be refused at pre-flight");

    assert!(
        matches!(err, MacosCuError::TenantMismatch { .. }),
        "expected TenantMismatch (existence-leak shape), got: {err:?}"
    );
}

/// **Positive**: a same-tenant `(tenant_a, worker_a)` request
/// passes the pre-flight gate. We only assert the gate's verdict;
/// the full `run()` would still fail downstream on macOS
/// permissions in CI, which is a separate gate.
#[tokio::test]
async fn same_tenant_worker_id_passes_pre_flight() {
    let setup = bootstrap_two_tenants().await;
    let lookup: Arc<dyn argo_executor_computer_use_macos::TenantWorkerLookup> =
        Arc::clone(&setup.store) as _;
    let executor = MacosCuExecutor::new(MacosCuConfig::for_notes_app(), lookup)
        .expect("construct executor with default config + lookup");

    let req = run_request(setup.tenant_a, setup.worker_a.id);
    executor
        .pre_flight_tenant_check(&req)
        .await
        .expect("same-tenant pair must pass the pre-flight gate");
}
