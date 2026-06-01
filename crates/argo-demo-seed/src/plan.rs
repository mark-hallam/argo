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
    /// Number of marketplace catalog rows (`skill` table) inserted
    /// under the demo tenant. The /skills SPA page reads this table
    /// via `GET /api/v1/skills` — fix-skills-marketplace-perpetual-
    /// loading requires ≥6 so the FEATURED panel paints real cards.
    pub marketplace_skills_created: usize,
    /// Number of `worker_skill` rows assigned (hero workers learning real
    /// catalog skills) so Claim 4's `/skills/<id>/usage` + "learned this
    /// month" surfaces show genuine adoption rather than an empty state.
    pub worker_skills_assigned: usize,
    /// Number of `role_duty` rows seeded for the hero workers so each
    /// worker overview's Responsibilities / Schedules card
    /// (`GET /api/v1/workers/<id>/overview` → `duties`) renders real
    /// scheduler rows instead of the "No schedules yet" empty state.
    /// Mariah gets the four Scene-2 runbook responsibilities.
    pub worker_duties_seeded: usize,
    /// Number of committed-retrain audit rows seeded for the lead hero
    /// workers so the worker Overview tab's "Recent changes" panel
    /// (`GET /api/v1/workers/<id>/retrain/recent`) renders real history
    /// inside the 14-day undo window instead of an empty mock.
    pub retrain_records_seeded: usize,
    /// Number of `audit_record` rows produced by Pass A real-LLM
    /// turns. Always `0` when [`SeedOptions::skip_pass_a_turns`]
    /// is true.
    pub pass_a_audit_rows: usize,
    /// Number of `audit_record` rows produced by Pass B (dressing)
    /// inserts.
    pub pass_b_audit_rows: usize,
    /// Number of `audit_record` rows produced by the action-type
    /// variety backstory pass (see `audit_backstory`). These rows
    /// give `/audit` ≥5 distinct displayed action_type values so the
    /// page reads as a working AI workforce rather than HTTP self-
    /// traffic spam (UX audit F3 / P1).
    pub audit_backstory_rows: usize,
    /// Number of `audit_record` rows inserted under the foreign
    /// tenant (Scene 6 invisibility check).
    pub foreign_audit_rows: usize,
    /// Number of `approval_request` rows the demo's Scene 2 Slack
    /// approval beat drills into.
    pub approval_requests: usize,
    /// Number of `retro` rows the demo seed inserts so `/retro` no
    /// longer renders "0 retros across the tenant — No retros yet"
    /// during investor walkthroughs beyond the 6-scene runbook
    /// (UX audit whole-site F08). The canonical seed inserts 3
    /// retros bound to Mariah / Priya / Peta, each with a 5-section
    /// narrative referencing the `customer_research_followup`
    /// workflow seeded by [`crate::workflow`].
    pub retros_created: usize,
    /// Number of orchestrator-provider `session` rows the demo seed
    /// inserts so `/orchestration` no longer renders "No orchestration
    /// yet" during investor walkthroughs beyond the 6-scene runbook
    /// (UX audit whole-site F08). The canonical seed inserts 2 sessions
    /// (Mariah-led marketplace re-price + Peta-led onboarding handoff).
    pub orchestration_runs_created: usize,
    /// Number of demo catalog skills given a real `skill_ir` graph so
    /// `GET /api/v1/skills/<id>/ir` returns a valid IR (the canvas
    /// viewer) instead of 404 `"skill has no IR"`. One graph per
    /// marketplace catalog skill, written via the real canvas writer
    /// (`argo_skill::ir::save_edited_graph`) so the hash + shape are
    /// valid. See [`crate::seed_skill_ir`].
    pub skill_ir_seeded: usize,
    /// Number of `skill_reeval_history` rows inserted for the demo
    /// catalog skills so `GET /api/v1/workers/<id>/evals` shows a real
    /// pass/fail trend (a worker's evals = the reeval history of its
    /// learned skills) instead of `{latest:null,history:[]}`. Three
    /// rows per catalog skill with varied pass rates + one regression.
    /// See [`crate::seed_skill_reeval_history`].
    pub skill_reeval_rows_seeded: usize,
    /// Number of demo catalog skills turned into a clean `v1`→`v2`→`v3`
    /// version chain (active head + two superseded history rows) so the
    /// /skills detail "Version History" panel
    /// (`GET /api/v1/skills/<id>/versions`) shows a coherent progression
    /// aligned with the Eval Re-run History versions — instead of a lone
    /// `0.1.0` row that joins no eval runs. Idempotent (`0` on a re-seed
    /// where the chain already exists). See
    /// [`crate::seed_skill_version_chain`].
    pub skill_version_chains_built: usize,
    /// `true` when the run drove real LLM turns; `false` when Pass A
    /// was skipped via [`SeedOptions::skip_pass_a_turns`].
    pub pass_a_real_llm: bool,
    /// Number of broadcast `notifications` rows seeded so the
    /// bell-popover (`GET /api/v1/notifications`) + unread-count badge
    /// render real rows instead of the SPA's "Nothing new." empty
    /// state (and the `SAMPLE_NOTIFICATIONS` fallback never fires).
    /// The canonical seed inserts 5 rows mirroring seeded approvals /
    /// workers / workflow runs. See [`crate::seed_notifications`].
    pub notifications_seeded: usize,
    /// Number of `workflow_run` rows seeded for the
    /// `customer_research_followup` workflow so the Scene-5 detail
    /// page's run-history aggregates + recent-runs table
    /// (`GET /api/v1/verticals/<v>/workflows/<n>/runs`) populate from
    /// real rows. The canonical seed inserts 7 runs. See
    /// [`crate::seed_workflow_runs`].
    pub workflow_runs_seeded: usize,
    /// Number of `workplace_knowledge_doc` rows seeded so
    /// `/settings/knowledge` (`GET /api/v1/knowledge`) lists real
    /// handbook pages instead of an empty state. The canonical seed
    /// inserts 4 docs (one per category). See [`crate::seed_knowledge`].
    pub knowledge_docs_seeded: usize,
    /// Number of hero workers given an honest non-zero
    /// `worker.trust_grade` so the worker-detail Trust tab
    /// (`GET /api/v1/workers/<id>/trust-grade`) shows a real grade
    /// instead of the `draft`-default unreviewed floor. See
    /// [`crate::seed_trust_grades`].
    pub trust_grades_set: usize,
    /// Number of metadata-only `api_key` rows seeded so
    /// `/settings/api-keys` (`GET /api/v1/api-keys`) shows a real key
    /// roster instead of an empty state. The canonical seed inserts 3
    /// rows ("Production CI" / "Staging dashboard" / "Analytics
    /// read-only") with random throwaway hashes — NOT working
    /// credentials, no plaintext secret. See [`crate::seed_api_keys`].
    pub api_keys_seeded: usize,
    /// Number of real memory facts written into the hero workers' own
    /// `WorkerSelf` namespaces (via the live
    /// [`argo_memory::MemoryStore::append_in_namespace`] path) so the
    /// worker Memory tab's "Facts she knows" panel
    /// (`GET /api/v1/workers/<id>/memory` → `facts`) shows real,
    /// persona-tied knowledge instead of an empty state. Idempotent per
    /// worker. See [`crate::seed_worker_memory`].
    pub worker_memory_facts_seeded: usize,
    /// Number of open conversation threads seeded into the
    /// `worker_conversation_thread` store (migration v87) so the Memory
    /// tab's "Active conversations" panel (`→ active_conversations`)
    /// renders real persona-tied threads. Idempotent per worker. See
    /// [`crate::seed_worker_memory`].
    pub worker_conversation_threads_seeded: usize,
    /// Number of non-canonical (stray) workers pruned — archived — from
    /// the demo tenant for record-readiness, so a freshly-seeded tenant is
    /// EXACTLY the canonical roster + enrichment. Runs ONLY on the demo
    /// path (`run_canonical_v1_seed`); canonical workers are never touched.
    /// See [`crate::seed_prune`].
    pub workers_pruned: usize,
}
