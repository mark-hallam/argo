//! Dept-filler worker seeder — minimum-seed coverage for the
//! `/cohorts` SPA page so all 9 department tabs declared in
//! `web/components/skycot/cohorts/dept-registry.ts` render with at
//! least one cohort each. The camera does NOT drill into these during
//! the v1.0 demo MP4; they exist for the dept-tab pan-over only. See
//! brief `fix-demo-seed-add-pitch-agent-and-missing-cohort-rows` and
//! memory `project_track_p_v1_mvp_scope` for the Sales & Investor
//! Relations (pitch-agent) row's load-bearing reason.

use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_types::{ArgoResult, TenantId};

use crate::personas::DEPT_FILLER_PERSONAS;

/// Insert the four dept-filler personas under `tenant_id`. Idempotent:
/// a re-seed returns the existing worker ids in the same order
/// without inserting duplicate rows, mirroring `seed_demo_workers`.
///
/// # Errors
///
/// Returns any identity-store or underlying SQLite error.
pub(crate) async fn seed_dept_filler_workers(
    identities: &IdentityStore<'_>,
    tenant_id: TenantId,
) -> ArgoResult<Vec<WorkerId>> {
    let existing = identities.list_workers(tenant_id, false).await?;
    let mut ids = Vec::with_capacity(DEPT_FILLER_PERSONAS.len());
    for spine in DEPT_FILLER_PERSONAS {
        if let Some(worker) = existing.iter().find(|w| w.persona.name == spine.name) {
            ids.push(worker.id);
            continue;
        }
        let persona = Persona::named(spine.name, "argo-demo-seed");
        let role = Role::new(format!(
            "{}: {}. Traits: {}.",
            spine.title,
            spine.tagline,
            spine.traits.join(", ")
        ));
        let worker = Worker::draft(tenant_id, spine.vertical_id, persona, role);
        let worker_id = worker.id;
        identities.create_worker(&worker).await?;
        ids.push(worker_id);
    }
    Ok(ids)
}
