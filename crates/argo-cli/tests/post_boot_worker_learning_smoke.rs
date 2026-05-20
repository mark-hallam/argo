//! Stream `mission-audit-skill-learning-dynamic-enroll` — proves that
//! workers hired AFTER gateway boot still join the per-worker
//! skill-learning cron, without waiting for a gateway restart.
//!
//! ## What this proves
//!
//! Pre-brief, `argo-cli/src/handlers/gateway/boot_serve.rs` enumerated
//! `worker WHERE archived_at IS NULL` once at boot and spawned a
//! [`argo_skill::spawn_per_worker_learning_cron`] handle per row.
//! Workers hired afterwards never joined the loop until the next
//! gateway boot — a regression of mission claim 4 ("skills shared and
//! learned") for post-boot hires.
//!
//! Post-brief, [`argo_identity::IdentityStore::create_worker`] carries
//! a [`argo_identity::LearningEnrollFn`] hook attached via
//! [`argo_identity::IdentityStore::with_learning_enroll`]; the gateway
//! hire HTTP handler attaches a closure backed by a process-global
//! [`argo_skill::WorkerLearningRegistry`]. Tests below exercise the
//! same identity-store seam directly (the HTTP boundary is a thin
//! shim that wires the same closure in `routes_workers_hire`).
//!
//! ## Three assertions
//!
//! 1. `hire_enrolls_worker_in_learning_registry` — the registry
//!    receives the new worker_id on `create_worker_from_draft`.
//! 2. `enroll_is_idempotent_on_duplicate_worker` — re-enrolling the
//!    same `worker_id` returns `false` and does not double-spawn a
//!    cron handle (count stays at 1).
//! 3. `no_enroll_when_registry_not_attached` — the back-compat path
//!    (`[skill_learning].enabled = false`) leaves the closure unset,
//!    `create_worker_from_draft` succeeds, and the registry stays
//!    empty (mirrors the boot path that returns `None` from
//!    `spawn_skill_learning`).

use argo_identity::{
    ApprovalPolicyChoice, HireDraft, HireDutyDraft, IdentityStore, LearningEnrollFn,
};
use argo_skill::{PerWorkerLearningConfig, SkillStore, WorkerLearningRegistry};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;
use std::time::Duration;

const LONG_INTERVAL: Duration = Duration::from_secs(3600);

async fn fresh_session_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

fn fresh_registry(store: &Arc<SessionStore>) -> Arc<WorkerLearningRegistry> {
    let skills = Arc::new(SkillStore::new(Arc::clone(store)));
    Arc::new(WorkerLearningRegistry::new(
        skills,
        PerWorkerLearningConfig::default(),
        LONG_INTERVAL,
    ))
}

fn enroll_fn(registry: &Arc<WorkerLearningRegistry>) -> LearningEnrollFn {
    let registry = Arc::clone(registry);
    Arc::new(move |worker_id, tenant_id| {
        registry.enroll(worker_id, tenant_id);
    })
}

fn sample_draft() -> HireDraft {
    HireDraft {
        suggested_name: "Maya".into(),
        charter: "Operate the post-boot hire path so newly created workers join the learning loop.".into(),
        suggested_vertical: "gtm".into(),
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
        tagline: None,
    }
}

#[tokio::test]
async fn hire_enrolls_worker_in_learning_registry() {
    let store = fresh_session_store().await;
    let tenant = TenantId::new();
    let registry = fresh_registry(&store);

    let identity = IdentityStore::new(&store).with_learning_enroll(enroll_fn(&registry));
    let worker_id = identity
        .create_worker_from_draft(&sample_draft(), tenant, "system:test")
        .await
        .unwrap();

    assert!(
        registry.is_enrolled(worker_id),
        "newly hired worker should be enrolled in the learning registry",
    );
    assert_eq!(registry.enrolled_count(), 1);
}

#[tokio::test]
async fn enroll_is_idempotent_on_duplicate_worker() {
    let store = fresh_session_store().await;
    let registry = fresh_registry(&store);
    let worker_id = argo_store::WorkerId::new();
    let tenant = TenantId::new();

    assert!(registry.enroll(worker_id, tenant));
    assert!(
        !registry.enroll(worker_id, tenant),
        "re-enrolling the same worker_id must not double-spawn",
    );
    assert_eq!(registry.enrolled_count(), 1);
}

#[tokio::test]
async fn no_enroll_when_registry_not_attached() {
    // Mirrors the `[skill_learning].enabled = false` boot path: no
    // registry is installed, so `IdentityStore` carries no
    // `with_learning_enroll` closure and `create_worker_from_draft`
    // produces a worker row without touching any learning surface.
    let store = fresh_session_store().await;
    let tenant = TenantId::new();
    let registry = fresh_registry(&store);

    let identity = IdentityStore::new(&store); // no with_learning_enroll
    let worker_id = identity
        .create_worker_from_draft(&sample_draft(), tenant, "system:test")
        .await
        .unwrap();

    assert!(
        identity
            .get_worker(tenant, worker_id)
            .await
            .unwrap()
            .is_some(),
        "worker should still be persisted in the back-compat path",
    );
    assert!(
        !registry.is_enrolled(worker_id),
        "registry must stay empty when no enroll closure is attached",
    );
    assert_eq!(registry.enrolled_count(), 0);
}
