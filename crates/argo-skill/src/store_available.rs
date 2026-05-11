//! [`SkillStore::available_for_worker`] — trust-gated lookup of the
//! active skills a worker may surface.
//!
//! Split out of `store.rs` to keep that file under the 500-LOC HARD
//! RULE 6 ceiling (store.rs was at 430 LOC when this brief landed; an
//! inline method + its doc comment would push it close enough to the
//! ceiling to block the next addition). The method body is the SQL
//! filter described in `mission-skill-store-available-for-worker`:
//! a per-tenant, single-statement `WHERE (min_trust_grade IS NULL)
//! OR (? >= min_trust_grade)` join against the active skill set.

use crate::skill::Skill;
use crate::store::SkillStore;
use crate::store_lookups::RawSkill;
use crate::trust_reader::WorkerTrustReader;
use argo_types::{ArgoError, ArgoResult, TenantId};
use uuid::Uuid;

impl SkillStore {
    /// Active skills the worker is permitted to surface, given the
    /// trust ladder stored on `skill.min_trust_grade`. A `NULL` floor
    /// means "no restriction"; a numeric floor is compared against
    /// the worker's `trust_grade` (read via `trust_reader`).
    ///
    /// Filter rule: `(min_trust_grade IS NULL) OR
    /// (worker_trust_grade >= min_trust_grade)`. Both sides of the
    /// comparison use the same numeric ladder (see
    /// `crates/argo-store/migrations/50__worker_trust_grade.sql` and
    /// `54__skill_min_trust_grade.sql`). `worker.trust_grade` defaults
    /// to `0` (most restrictive); a fresh worker therefore only sees
    /// skills with `min_trust_grade IS NULL OR min_trust_grade <= 0`.
    ///
    /// Returns `ArgoError::Provider` when no worker matches the
    /// `(tenant_id, worker_id)` pair — the brief specifies a hard
    /// error rather than an empty list so an upstream typo /
    /// wrong-tenant surfaces loudly.
    ///
    /// Out of scope (per brief): cross-tenant filtering (this lookup
    /// is per-tenant by construction), workspace filtering, skill
    /// version selection (active rows only).
    pub async fn available_for_worker(
        &self,
        trust_reader: &dyn WorkerTrustReader,
        tenant_id: TenantId,
        worker_id: Uuid,
    ) -> ArgoResult<Vec<Skill>> {
        let grade = trust_reader
            .trust_grade(tenant_id, worker_id)
            .await?
            .ok_or_else(|| {
                ArgoError::Provider(format!(
                    "worker {worker_id} not found in tenant {tenant_id}"
                ))
            })?;
        let rows = sqlx::query_as::<_, RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, \
             s.hash, s.learned_in, s.superseded_by, s.created_at, l.lock, \
             s.vertical_id, s.lifecycle_state \
             FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
             WHERE s.superseded_by IS NULL \
             AND (s.min_trust_grade IS NULL OR ? >= s.min_trust_grade) \
             ORDER BY s.name ASC",
        )
        .bind(i64::from(grade))
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("available_for_worker: {e}")))?;
        rows.into_iter().map(Skill::try_from).collect()
    }
}
