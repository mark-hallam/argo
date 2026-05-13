# mission-operator-feedback-dreams-curation-v2 — operator feedback verbs on Dreams curation (delta over already-shipped Promote/Reject)

**Owner**: claude. **Depends on**: shipped `argo plan template promote` (`cli_plan_cmds.rs:97` → `learned_plan_template::promote:219`); shipped `learned_plan_template::reject:237`.
**Authored against**: origin/main `b9d61ba3` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-operator-feedback-dreams-curation-v2`.

> **REWRITE 2026-05-13 by conductor (CEO-authorized).** V1 (`mission-operator-feedback-dreams-curation`) proposed ~1100 LOC of new infrastructure — `DreamsFeedback` enum + `dreams_feedback` table + `argo plan-template feedback` CLI — but made zero mention of the already-shipped `argo plan template promote` + `learned_plan_template::reject` surfaces. Implementing V1 verbatim would build a parallel operator-feedback system on the same data model. V2 ships ONLY the genuinely-new pieces (~350 LOC).

## Scope

Half of the V1 brief is already on main:
- **Promote** semantics: `cli_plan_cmds.rs:97` `PlanTemplateCmd::Promote` → `learned_plan_template::promote(id)` flips `trust_grade` Pending → Promoted; planner picks up via existing `with_learned_templates`.
- **Retire** semantics: `learned_plan_template::reject(id):237` flips trust_grade to Rejected; planner already skips Rejected.

The **3 genuinely-new operator verbs** this brief adds:

1. **`argo plan template reject <id>`** CLI parity — the `reject` store helper exists but no CLI surface. ~30 LOC handler that mirrors the existing `Promote` handler.

2. **`argo plan template annotate <id> --note "..."`** — free-text operator notes on a template. NEW `learned_plan_template_note` table (single column: id, template_id, author, note, ts). NEW handler. NEW CLI variant. ~120 LOC.

3. **`argo plan template request-revision <id> --gap "..."`** — operator-requested revision injected into next curation prompt. Two parts:
   - CLI + store: NEW handler stores `revision_request` against the template. ~80 LOC.
   - **Curation-loop wiring** — modify `argo-skill::dreams_distillation::distill_cohort` to consult `revision_request` rows for the cohort's templates and include `gap_description` in the prompt context. ~70 LOC (this is the real "curation-loop feedback" feature).

4. **Audit-chain rows** for all 4 operator actions (promote / reject / annotate / request-revision). Reuse the existing `AuditSink`. ~30 LOC.

**Not in scope**: changing `TrustGrade` semantics; adding a parallel `DreamsFeedback` enum (V1's mistake); replacing the existing `PlanTemplateCmd::Promote` CLI handler; UI surfaces.

## Parallelism

- Manager-worker allowed: no
- Max children: 0; Max depth: 1

## Files you may touch

- `crates/argo-cli/src/cli_plan_cmds.rs` — narrowly: add `Reject`, `Annotate`, `RequestRevision` variants to `PlanTemplateCmd` (around line 73-110). Mirror existing `Promote` shape. ≤30 modified lines.
- `<NEW: crates/argo-cli/src/handlers/plan_template_reject.rs>` (≤50 LOC) — handler that calls existing `learned_plan_template::reject`.
- `<NEW: crates/argo-cli/src/handlers/plan_template_annotate.rs>` (≤120 LOC) — handler + new table insert.
- `<NEW: crates/argo-cli/src/handlers/plan_template_request_revision.rs>` (≤120 LOC) — handler + revision-request row insert.
- `<NEW: crates/argo-store/migrations/<slot>__learned_plan_template_feedback.sql>` (≤30 LOC) — ONE migration creates BOTH new tables (`learned_plan_template_note`, `learned_plan_template_revision_request`).
- `crates/argo-store/src/migrations/registry.rs` — register the new slot.
- `<NEW: crates/argo-store/src/learned_plan_template_feedback.rs>` (≤150 LOC) — `insert_note`, `insert_revision_request`, `fetch_pending_revisions_for_cohort` helpers + unit tests.
- `crates/argo-store/src/lib.rs` — narrowly: re-export the new module.
- `crates/argo-skill/src/dreams_distillation.rs` — narrowly: at the cohort-prompt construction site, consult `fetch_pending_revisions_for_cohort` and inject `gap_description` into the prompt. ≤70 modified lines.
- `crates/argo-cli/src/main.rs` — narrowly: dispatch the 3 new subcommands. ≤15 modified lines.

## Files you must NOT touch

- The existing `argo plan template promote` handler / `learned_plan_template::promote` — locked.
- `learned_plan_template::reject` — locked; CLI wraps it.
- `TrustGrade` enum — locked.
- The planner's `with_learned_templates` pickup logic — locked; already correctly skips Rejected.
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] `argo plan template reject <id>` flips trust_grade to Rejected (call `learned_plan_template::reject`).
- [ ] `argo plan template annotate <id> --note "..."` inserts a row in `learned_plan_template_note`; template's `trust_grade` UNCHANGED (annotations are context-only).
- [ ] `argo plan template request-revision <id> --gap "..."` inserts in `learned_plan_template_revision_request`; trust_grade UNCHANGED.
- [ ] `distill_cohort` consults pending revision requests for the cohort's templates and includes `gap_description` strings in the curation prompt context.
- [ ] Audit-chain rows emitted for all 4 operator verbs (promote already shipped; this brief wires reject + annotate + request-revision audit emission; promote stays unchanged).
- [ ] 6 unit tests + 1 smoke (`argo plan template promote/reject/annotate/request-revision` end-to-end).
- [ ] No parallel `DreamsFeedback` enum / no `dreams_feedback` table — re-uses existing `TrustGrade` + adds 2 small adjunct tables.
- [ ] Additive migration only; slot picked from free pool (verify against `gh pr list --state open`).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-operator-feedback-dreams-curation-v2`
- PR title: `plan-template: reject/annotate/request-revision operator verbs (mission-operator-feedback-dreams-curation-v2)`

## Notes for the implementer

- The Promote/Reject pair is the existing pattern. New verbs follow the same handler shape.
- Annotations and revision-requests are **context-only** — they don't change trust_grade. The brief's intent is operator-curated metadata, not lifecycle transitions.
- The `request-revision` curation-loop wiring is the load-bearing-new feature. Without it, the operator's revision-gap text never reaches the next curation pass.
- Per CLAUDE.md "no parallel abstractions" — if you find yourself building a `DreamsFeedbackStore` trait, stop and re-read this brief.
- ~350 LOC total. Should ship in one PR.
