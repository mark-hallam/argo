//! Demo workspace seed for the SKYCOT v1.0 demo MP4.
//!
//! Stands up six canonical workers (Mariah · Priya · Jamal · Nori ·
//! Peta · Patrick) in a tenant via [`argo_identity::IdentityStore`].
//! No LLM turns, no approvals, no audit rows beyond what the
//! `create_worker` path already emits — those scopes belong to the
//! follow-up briefs `mission-demo-seed-real-llm-driver` (turns) and
//! `mission-demo-seed-runbook-integration` (CLI + runbook wiring).
//!
//! The seed is idempotent: re-running on the same tenant returns the
//! existing six worker IDs (not 12 duplicates). The idempotency key
//! is `(tenant_id, persona.name)`, checked against
//! [`argo_identity::IdentityStore::list_workers`].
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_demo_seed::seed_workers;
//! use argo_store::SessionStore;
//! use argo_types::TenantId;
//!
//! let store = SessionStore::open(":memory:").await?;
//! store.migrate().await?;
//! let tenant = TenantId::new();
//! let ids = seed_workers(&store, tenant).await?;
//! assert_eq!(ids.len(), 6);
//! # Ok(()) }
//! ```
//!
//! ## Sensible deviation from the brief
//!
//! The brief (`mission-demo-seed-crate-scaffold.md`) specifies
//! `seed_workers(deps: &AgentDeps, tenant_id: &TenantId)`. The
//! function only consumes the `SessionStore` half of `AgentDeps`
//! (no transport, capability, audit, or steer use), so this crate
//! takes `&SessionStore` directly. That drops an `argo-core`
//! dependency this scaffold would otherwise carry purely for the
//! type wrapper, and keeps the test setup at three lines
//! (`SessionStore::open` + `migrate` + `seed_workers`) instead of
//! ~30 lines of `AgentDeps` plumbing. Brief #2
//! (`mission-demo-seed-real-llm-driver`) takes `&AgentDeps` where
//! the transport + audit fields are actually used.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod personas;

pub use personas::{DemoPersona, DEMO_PERSONAS};

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::{SessionStore, WorkerId};
use argo_types::{ArgoResult, TenantId};

/// Builder attribution string stamped on every persona this seed
/// creates. Surfaces in the audit `worker.hired` row's
/// `builder_attribution` column so demo-seeded workers are
/// distinguishable from operator-hired workers in the audit feed.
pub const SEED_BUILDER_ATTRIBUTION: &str = "argo-demo-seed";

/// Seed the canonical six demo workers into `tenant_id`.
///
/// Returns the six [`WorkerId`]s in [`DEMO_PERSONAS`] order — stable
/// across runs, so callers can pin downstream rows (audit scenes,
/// approval queues, runbook timestamps) to specific personas by
/// index without re-querying by name.
///
/// # Errors
///
/// Returns any error from
/// [`IdentityStore::ensure_schema`],
/// [`IdentityStore::list_workers`], or
/// [`IdentityStore::create_worker`] — typically a SQLx provider
/// error if the underlying SQLite store is unreachable.
pub async fn seed_workers(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Vec<WorkerId>> {
    let identities = IdentityStore::new(store);
    identities.ensure_schema().await?;

    let existing = identities.list_workers(tenant_id, false).await?;

    let mut ids = Vec::with_capacity(DEMO_PERSONAS.len());
    for spec in DEMO_PERSONAS {
        if let Some(existing) = existing.iter().find(|w| w.persona.name == spec.name) {
            tracing::debug!(
                name = spec.name,
                worker_id = %existing.id,
                "argo-demo-seed: reusing existing worker",
            );
            ids.push(existing.id);
            continue;
        }
        let worker = build_worker(tenant_id, spec);
        let worker_id = worker.id;
        identities.create_worker(&worker).await?;
        tracing::info!(
            name = spec.name,
            vertical = spec.vertical_id,
            worker_id = %worker_id,
            "argo-demo-seed: created worker",
        );
        ids.push(worker_id);
    }
    Ok(ids)
}

fn build_worker(tenant_id: TenantId, spec: &DemoPersona) -> Worker {
    let mut persona = Persona::named(spec.name, SEED_BUILDER_ATTRIBUTION);
    persona.voice_profile.preamble = format!(
        "You are {name}, an autonomous SKYCOT worker. Role: {tagline}. \
         Personality: {traits}.",
        name = spec.name,
        tagline = spec.tagline,
        traits = spec.traits.join(" · "),
    );
    persona.bio = Some(spec.charter.to_string());

    let role = Role::new(spec.charter);
    Worker::draft(tenant_id, spec.vertical_id, persona, role)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_personas_spine_is_exactly_six_named_workers() {
        let names: Vec<&str> = DEMO_PERSONAS.iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            vec!["Mariah", "Priya", "Jamal", "Nori", "Peta", "Patrick"],
        );
    }

    #[test]
    fn demo_personas_have_non_empty_charters_and_traits() {
        for spec in DEMO_PERSONAS {
            assert!(
                !spec.charter.trim().is_empty(),
                "{} missing charter",
                spec.name
            );
            assert!(
                !spec.traits.is_empty(),
                "{} missing personality traits",
                spec.name
            );
            assert!(
                !spec.vertical_id.trim().is_empty(),
                "{} missing vertical_id",
                spec.name
            );
        }
    }

    #[tokio::test]
    async fn seed_workers_creates_six_workers_in_persona_order() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        let tenant = TenantId::new();

        let ids = seed_workers(&store, tenant).await.unwrap();
        assert_eq!(ids.len(), 6);

        let identities = IdentityStore::new(&store);
        let mut listed = identities.list_workers(tenant, false).await.unwrap();
        listed.sort_by_key(|w| w.persona.name.clone());
        let mut expected: Vec<String> =
            DEMO_PERSONAS.iter().map(|p| p.name.to_string()).collect();
        expected.sort();
        assert_eq!(
            listed.iter().map(|w| w.persona.name.clone()).collect::<Vec<_>>(),
            expected,
        );
    }

    #[tokio::test]
    async fn seed_workers_is_idempotent() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        let tenant = TenantId::new();

        let first = seed_workers(&store, tenant).await.unwrap();
        let second = seed_workers(&store, tenant).await.unwrap();
        assert_eq!(first, second, "re-running must return the same ids");

        let identities = IdentityStore::new(&store);
        let listed = identities.list_workers(tenant, false).await.unwrap();
        assert_eq!(listed.len(), 6, "re-seed must not duplicate rows");
    }
}
