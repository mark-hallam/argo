//! Options + result types for [`crate::run_canonical_v1_seed`].

use argo_types::TenantId;

/// Inputs to [`crate::run_canonical_v1_seed`].
#[derive(Debug, Clone)]
pub struct SeedOptions {
    /// Tenant id to seed the demo workspace under. Must point at an
    /// already-provisioned tenant — the seed does not create the
    /// tenant row itself.
    pub tenant_id: TenantId,
    /// Profile id selecting which seed body to run. Currently only
    /// `"v1-demo-canonical"` is shipped; future profiles will hang
    /// off this string.
    pub profile: String,
    /// Number of Pass B (dressing) historical audit rows to insert.
    /// Default 150 (within the 100-200 band the brief calls for).
    /// Capped at 500 to keep one-shot runs bounded.
    pub dressing_audit_rows: usize,
    /// Number of Pass A (function-side) real-LLM agent turns to drive
    /// once a provider key is configured. The brief calls for 10-20.
    /// Capped at 50.
    pub pass_a_turns: usize,
    /// When `true`, skip the Pass A real-LLM turn phase entirely and
    /// land only the scaffolding (workers + workflow def + skill
    /// catalog + cross-tenant rows) plus Pass B dressing rows. The
    /// CLI surfaces this as `--skip-pass-a-turns`; useful for smoke
    /// tests and for the operator who wants to seed offline without
    /// burning API credits.
    pub skip_pass_a_turns: bool,
    /// Free-text id stamped into every row's `created_by` / actor
    /// column. Defaults to `"argo-demo-seed"` so the audit feed
    /// shows a recognisable origin.
    pub actor: String,
}

impl SeedOptions {
    /// Build a default options bundle for the canonical v1 profile.
    #[must_use]
    pub fn canonical_v1(tenant_id: TenantId) -> Self {
        Self {
            tenant_id,
            profile: crate::DEFAULT_PROFILE.to_string(),
            dressing_audit_rows: 150,
            pass_a_turns: 12,
            skip_pass_a_turns: false,
            actor: "argo-demo-seed".to_string(),
        }
    }
}

/// Counts surfaced by [`crate::run_canonical_v1_seed`] so the CLI can
/// print a human-readable transcript at the end of a seed run.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct SeedReport {
    /// Number of `worker` rows inserted under the demo tenant.
    pub workers_created: usize,
    /// Number of `worker` rows inserted under the foreign tenant
    /// (used by Scene 6's cross-tenant invisibility beat).
    pub foreign_workers_created: usize,
    /// Number of `workflow_definition` rows inserted under the demo
    /// tenant. The canonical seed inserts exactly one
    /// (`customer_research_followup`).
    pub workflows_created: usize,
    /// Number of `skill_ir` rows inserted under the demo tenant.
    /// The canonical seed inserts the
    /// `competitor pricing scrape` skill plus two siblings.
    pub skills_created: usize,
    /// Number of `skill_ir` rows inserted under the foreign tenant
    /// (Scene 6 invisibility check).
    pub foreign_skills_created: usize,
    /// Number of `audit_record` rows produced by Pass A real-LLM
    /// turns. Always `0` when [`SeedOptions::skip_pass_a_turns`]
    /// is true.
    pub pass_a_audit_rows: usize,
    /// Number of `audit_record` rows produced by Pass B (dressing)
    /// inserts.
    pub pass_b_audit_rows: usize,
    /// Number of `audit_record` rows inserted under the foreign
    /// tenant (Scene 6 invisibility check).
    pub foreign_audit_rows: usize,
    /// Number of `approval_request` rows the demo's Scene 2 Slack
    /// approval beat drills into.
    pub approval_requests: usize,
    /// `true` when the run drove real LLM turns; `false` when Pass A
    /// was skipped via [`SeedOptions::skip_pass_a_turns`].
    pub pass_a_real_llm: bool,
}
