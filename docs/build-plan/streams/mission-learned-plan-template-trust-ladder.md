# Mission load-bearing wire — LearnedPlanTemplate ingestion into TrustGrade ladder

**Owner**: claude. **Depends on**: `argo-store::learned_plan_template::{LearnedPlanTemplateRow, insert, promote, reject, list_promoted}` SHIPPED (verified at `crates/argo-store/src/learned_plan_template.rs:69,151,180`); `argo-skill::trust::TrustGrade` SHIPPED (verified at `crates/argo-skill/src/trust.rs:33`); existing skill-trust-grade column SHIPPED.
**Authored against**: origin/main `d50e69b2` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-learned-plan-template-trust-ladder`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 #89 — "Build LearnedPlanTemplate ingestion into TrustGrade ladder." Today learned plan templates from Dreams sit in their own table with `promoted` / `rejected` lifecycle but they don't participate in the `TrustGrade` ladder that controls skill propagation. Mission promise: when a learned plan template is promoted (via `promote()` at `learned_plan_template.rs:180`), it ALSO synthesises a `Skill` row at `TrustGrade::Tenant` for the same tenant — so it shows up in `argo skill list` and propagates to workers per the mission claim-4 wiring.

## Scope

1. **NEW `argo-skill::ingest::from_learned_plan_template(row, skill_store, trust_grade) -> ArgoResult<SkillId>`**:
   - Maps `LearnedPlanTemplateRow` fields to a `Skill` insert (description, body, keywords).
   - The skill `name` = `"learned:" + row.template_id` (namespace prefix prevents collision with hand-authored skills).
   - Trust grade defaults to `TrustGrade::Tenant`; caller can override (used by tests).
   - Idempotent: re-ingesting the same template_id updates the existing skill row, never creates a dup.

2. **Hook into `promote()`**: extend `learned_plan_template::promote(pool, id)` to ALSO call `from_learned_plan_template` via a NEW `SkillIngestSink` trait passed at promotion time. This avoids `argo-store → argo-skill` direct dep — caller threads the sink.

3. **Trait**: NEW `SkillIngestSink` in `argo-store::learned_plan_template`:
   ```rust
   #[async_trait::async_trait]
   pub trait SkillIngestSink: Send + Sync {
       async fn ingest_from_template(&self, row: &LearnedPlanTemplateRow)
           -> ArgoResult<()>;
   }
   ```
   Default impl (used in unit tests): no-op. Concrete impl in `argo-cli` wraps the actual `SkillStore`.

4. **CLI hook**: existing `argo dreams promote <id>` command (verify path) — narrowly scoped: instantiate the concrete sink + pass through. NO change to existing flags.

5. **Reject path**: `learned_plan_template::reject(pool, id)` — narrowly scoped: if a skill was previously ingested (lookup by `name = "learned:" + id`), demote its trust grade to `TrustGrade::Untrusted` (NEW variant if not present, else lowest existing). The skill is NOT deleted — operator might want to undo.

6. **Operator query**: `argo skill list --learned [--json]` (NEW filter) — list ONLY learned skills. Useful for "what did Dreams promote into my catalog?"

This is NOT in scope: cross-tenant learned-skill propagation (`mission-cross-tenant-trust-graph` owns that); per-template ranking; bulk re-ingest after migrations; auto-demote on negative feedback.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-skill/src/ingest_learned.rs` (NEW, ≤ 250 LOC) — `from_learned_plan_template` + helpers.
- `crates/argo-skill/src/ingest_learned_tests.rs` (NEW, sibling, ≤ 200 LOC) — 6 unit tests: fresh-ingest-creates-skill; re-ingest-updates-not-dup; namespace-prefix-in-name; trust-grade-defaults-tenant; reject-demotes-not-deletes; reject-when-no-skill-no-op.
- `crates/argo-skill/src/lib.rs` — narrowly scoped: re-export `ingest_learned::*`.
- `crates/argo-store/src/learned_plan_template.rs` — narrowly scoped: extend `promote` + `reject` to accept `&dyn SkillIngestSink`. ADD `SkillIngestSink` trait. ≤ 60 LOC delta (file currently ~280 LOC; stay under 500).
- `crates/argo-store/src/learned_plan_template_tests.rs` — narrowly scoped: 3 new tests covering the sink integration.
- `crates/argo-cli/src/handlers/dreams_promote.rs` (verify path; may be `dreams.rs` with a `Promote` arm) — narrowly scoped: instantiate concrete `SkillIngestSink` impl. ≤ 30 LOC delta.
- `crates/argo-cli/src/handlers/skill_list.rs` (verify path) — narrowly scoped: ADD `--learned` filter flag. ≤ 25 LOC delta.

## Files you must NOT touch

- `crates/argo-skill/src/trust.rs` — TrustGrade enum stable; reuse only (NEW `Untrusted` variant only if not present — verify first; if absent, this is a precursor brief)
- `crates/argo-skill/src/store.rs` — reuse `install` / `update` methods only
- `crates/argo-store/migrations/**` — no schema change (both tables already exist)
- `crates/argo-runtime/src/dreams_plan_templates.rs` — Dreams generation surface stable; reuse only
- Any non-skill/non-store/non-cli crate

## Acceptance criteria

- [ ] `from_learned_plan_template` exists in `argo-skill::ingest`
- [ ] Promoted templates appear as skills with `name = "learned:<id>"`
- [ ] Re-promote of same template updates the existing skill row (no dup)
- [ ] Default trust grade is `TrustGrade::Tenant`
- [ ] `reject()` demotes the corresponding skill to lowest grade (not deletes)
- [ ] `SkillIngestSink` trait + no-op default impl exist
- [ ] `argo dreams promote <id>` wires the concrete sink
- [ ] `argo skill list --learned` filters to `learned:*` names
- [ ] 6+3 unit tests + 1 integration smoke (promote → skill list --learned shows it)
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-skill -p argo-store -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-learned-plan-template-trust-ladder`
- PR title: `skill: ingest learned plan templates into trust ladder on Dreams promote (mission-learned-plan-template-trust-ladder)`

## Notes for the implementer

- The `SkillIngestSink` indirection is load-bearing: it keeps `argo-store` free of an `argo-skill` dep cycle. The concrete impl lives in `argo-cli` where both crates are in scope.
- The `"learned:"` namespace prefix is a deliberate naming convention — downstream tools can pattern-match to distinguish hand-authored from learned skills.
- If `TrustGrade::Untrusted` doesn't exist yet, BLOCK and request a precursor brief — don't silently add a new enum variant in this PR.
- The `argo skill list --learned` filter is a small UX win; if you find the existing `list` handler is fragile, defer this to a follow-up.
- Idempotency is critical — operators who re-run `dreams promote` shouldn't get duplicate skill rows.
