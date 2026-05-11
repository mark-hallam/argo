# Mission load-bearing wire — operator-feedback channel into Dreams curation

**Owner**: claude. **Depends on**: existing `crates/argo-skill/src/dreams_distillation.rs` (Dreams curation pipeline); existing `crates/argo-runtime/src/dreams_plan_templates.rs` (LearnedPlanTemplate output); `mission-planner-bias-from-template` brief (parallel — both consume Dreams; coordinate so this brief doesn't preempt that one's API surface).
**Authored against**: origin/main `c26e63e1` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-operator-feedback-dreams-curation`.

> SOURCE: MISSION_BACKLOG.md Section 5 Claim 7-9 #197 ("Build operator-feedback channel into Dreams curation"). **Tier M (self-improvement-infra).** Today: Dreams curation runs autonomously — it ingests trajectory samples, distills patterns, promotes templates. Operators can SEE the resulting templates (via `argo plan-template list`) but have NO channel to feed back "this template is great" / "this template is harmful, retire" / "this template is missing a fix-up step" into the curation loop. This brief lands the operator-feedback API + storage + the curation pipeline's consumption of it.

## Scope

NEW `crates/argo-runtime/src/dreams_feedback.rs` (≤ 200 LOC):

- `pub enum DreamsFeedback` — 4 variants:
  - `Promote { template_id, reason: String, by_operator_id }` — operator says "use this template MORE — increase trust-grade ceiling"
  - `Retire { template_id, reason: String, by_operator_id }` — operator says "stop biasing toward this template; trust-grade floor = 0"
  - `Annotate { template_id, note: String, by_operator_id }` — operator adds context for future curation (free-text, ≤ 500 chars)
  - `RequestRevision { template_id, gap_description: String, by_operator_id }` — operator says "this template is missing X; next curation pass should consider X"
- `pub struct DreamsFeedbackEntry { id, template_id, kind, reason_or_note, by_operator_id, ts, applied: bool, applied_ts: Option<...> }`
- `pub trait DreamsFeedbackStore: Send + Sync` w/ `record(entry: DreamsFeedbackEntry) -> ArgoResult<()>`, `list_for_template(template_id) -> Vec<DreamsFeedbackEntry>`, `list_unapplied() -> Vec<DreamsFeedbackEntry>`, `mark_applied(id) -> ArgoResult<()>`.
- Default `StoreBackedDreamsFeedback` impl wrapping the existing argo-store path.
- 6 unit tests covering: all 4 variants record correctly, list-by-template returns correct subset, list-unapplied filters by `applied = false`, mark-applied is idempotent, free-text length cap enforced (Annotate / RequestRevision both ≤ 500 chars), feedback by-different-operators on same template all preserved.

### Migration v59+/SQL 63+

NEW `crates/argo-store/src/migrations/vNN.rs` (verify slot — as of brief authoring v58/SQL 62 is the live tail; coordinate w/ parallel `mission-planner-bias-from-template` / `mission-gateway-request-audit-log` / `mission-audit-chain-watchdog` / `mission-skill-explanation-provenance` briefs in this slot range).

```sql
CREATE TABLE IF NOT EXISTS dreams_feedback (
    id              TEXT    PRIMARY KEY,           -- ULID-style
    template_id     TEXT    NOT NULL,
    kind            TEXT    NOT NULL,               -- "Promote" | "Retire" | "Annotate" | "RequestRevision"
    reason_or_note  TEXT    NOT NULL,
    by_operator_id  TEXT    NOT NULL,
    ts              INTEGER NOT NULL,
    applied         INTEGER NOT NULL DEFAULT 0,
    applied_ts      INTEGER
);
CREATE INDEX IF NOT EXISTS idx_dreams_feedback_template ON dreams_feedback(template_id);
CREATE INDEX IF NOT EXISTS idx_dreams_feedback_unapplied ON dreams_feedback(applied) WHERE applied = 0;
```

### CLI

EXTEND `crates/argo-cli/src/cli_command_enum.rs` (≤ 20 LOC delta) — new `PlanTemplateCmd::Feedback` variant:
```
argo plan-template feedback <template_id> --kind <Promote|Retire|Annotate|RequestRevision> --reason "..."
argo plan-template feedback list [--template <id>] [--unapplied-only]
argo plan-template feedback apply <feedback_id>
```

NEW `crates/argo-cli/src/handlers/plan_template_feedback.rs` (NEW, ≤ 165 LOC).

### Dreams-curation pipeline consumption

EXTEND `crates/argo-skill/src/dreams_distillation.rs` (≤ 50 LOC delta — surgical, not a rewrite):

- BEFORE promoting any new template, query `DreamsFeedbackStore::list_for_template(template_id)`:
  - Any `Retire` entry → SKIP promotion (template stays at current trust-grade or below)
  - Any `Promote` entry → bias the promotion threshold by `-1 TrustGrade` (easier to promote — operator-vouched)
  - Any `RequestRevision` entry → annotate the next curation cycle's input prompt w/ the `gap_description` (operator-specified pattern injection)
  - `Annotate` entries are passed through as context only (not promotion-affecting)
- After each curation pass, `mark_applied` every entry that was consumed.

### Audit-chain emission

Every `DreamsFeedback` action emits an audit-chain row (`kind = "dreams_feedback_recorded"`, body `{ template_id, kind, by_operator_id }`). Every curation-pipeline consumption emits `dreams_feedback_applied` w/ the same shape + the resulting promotion-decision.

### Tests

8 unit tests in `dreams_feedback_tests.rs` (≤ 170 LOC) + 2 integration tests in `crates/argo-cli/tests/dreams_feedback_smoke.rs` (≤ 180 LOC): full CLI flow (record → list → apply → curation consumption asserted).

NO change to `dreams_plan_templates.rs` data model. NO change to the planner-bias path (that's `mission-planner-bias-from-template`'s scope; this brief's `Retire` semantically reduces the bias-path's TrustGrade query result, but the implementation is via `dreams_distillation`'s gating).

This is NOT in scope: a feedback-UI surface (CLI + JSON in v1; web UI is a separate productization brief #200-series); ML-driven sentiment-analysis of operator notes (the `RequestRevision` gap_description is passed as raw context to the next curation prompt; auto-classification is v2); cross-tenant feedback aggregation (per-tenant isolation only); operator-feedback on Forge runs (different surface — Forge has its own judge feedback path); rollback of an applied Promote/Retire (operators issue a counter-feedback; appends-only audit trail); auto-expiry of stale feedback (operator manages cleanup; auto-expiry is v2).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-runtime/src/dreams_feedback.rs` (NEW, ≤ 200 LOC)
- `crates/argo-runtime/src/dreams_feedback_tests.rs` (NEW, ≤ 170 LOC)
- `crates/argo-runtime/src/lib.rs` (EXTEND ≤ 5 LOC — re-export)
- `crates/argo-skill/src/dreams_distillation.rs` (EXTEND ≤ 50 LOC — gating consumption)
- `crates/argo-store/src/dreams_feedback_store.rs` (NEW, ≤ 180 LOC) — typed store impl
- `crates/argo-store/src/dreams_feedback_store_tests.rs` (NEW, ≤ 140 LOC)
- `crates/argo-store/src/migrations/vNN.rs` (NEW; verify slot at claim time — v59+ at brief author time)
- `crates/argo-store/migrations/<NN>__dreams_feedback.sql` (NEW; verify slot ≥ 63)
- `crates/argo-store/src/migrations/mod.rs` (EXTEND — register migration)
- `crates/argo-cli/src/cli_command_enum.rs` (EXTEND ≤ 20 LOC)
- `crates/argo-cli/src/handlers/plan_template_feedback.rs` (NEW, ≤ 165 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (EXTEND ≤ 3 LOC)
- `crates/argo-cli/tests/dreams_feedback_smoke.rs` (NEW, ≤ 180 LOC)
- `docs/skycot/runbook/dreams-feedback.md` (NEW, ≤ 180 LOC) — operator runbook

## Files you must NOT touch

- `crates/argo-runtime/src/dreams_plan_templates.rs` (data model stays; feedback is a sibling table)
- `crates/argo-forge/**` (orthogonal to Forge feedback)
- Any vertical manifest

## Acceptance criteria

- [ ] `cargo test -p argo-runtime dreams_feedback` passes all 6 + 8 unit tests
- [ ] `cargo test -p argo-cli --test dreams_feedback_smoke` passes both integration tests
- [ ] `argo plan-template feedback <id> --kind Promote --reason "..."` records correctly
- [ ] `argo plan-template feedback list --unapplied-only` returns only `applied=0` entries
- [ ] After a curation pass, all consumed feedback entries are marked `applied=true` w/ correct `applied_ts`
- [ ] `Retire` entry prevents new promotions of that template (smoke verifies)
- [ ] `Promote` entry biases threshold by -1 TrustGrade (easier promotion)
- [ ] `RequestRevision.gap_description` is appended to the curation prompt for the next pass
- [ ] Free-text fields capped at 500 chars (length-check enforced)
- [ ] Audit-chain rows emitted for every record + every apply
- [ ] Migration v59+/SQL 63+ adds dreams_feedback table idempotently
- [ ] All new files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-operator-feedback-dreams-curation`
- PR title: `runtime: Dreams operator-feedback channel (promote / retire / annotate / request-revision) + CLI (mission-operator-feedback-dreams-curation)`

## Notes for the implementer

- The 4 feedback kinds are the load-bearing API. Don't add a 5th in v1 — operators will discover what they need via usage.
- Append-only model: applying a feedback entry doesn't delete it; just sets `applied=true`. Audit trail is complete.
- The CLI is the v1 interface. A web UI surface (`/plan-templates/<id>/feedback`) is a follow-up brief — keep the JSON envelope stable so future surfaces can consume it.
- Free-text cap of 500 chars matches the existing operator-note pattern (verify against other operator-typed fields in argo-store — match for consistency).
- Migration coordination is critical: there are 5+ parallel briefs reserving slots v59 / SQL 63 onwards. At claim time, run `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -5` AND `ls crates/argo-store/migrations/*.sql | sort -V | tail -5` and pick the next free pair.
- The `Retire` semantic doesn't DELETE the template — it sets a flag the curation pipeline observes. Operators don't lose history; they bias future curation.
- DO NOT auto-apply feedback during the recording-CLI call. The curation pass is the consumer; recording is a queue-write. Separating these makes timing predictable.
- DO NOT add cross-tenant feedback (Tenant A's operator's feedback on Template X affecting Tenant B's curation). Tenancy isolation is load-bearing — feedback is per-tenant.
- DO NOT support "anonymous" feedback. `by_operator_id` is required for accountability + audit.
- The runbook should walk through one example end-to-end: "you notice template `tpl_abc` produces verbose plans → `argo plan-template feedback tpl_abc --kind Retire --reason 'verbose'` → next curation pass skips promotion → audit-chain row recorded."
- `RequestRevision.gap_description` is passed as plain text to the curation prompt. Don't try to structure it — that's a v2 schema-ification once usage patterns emerge.
- Coordinate w/ `mission-planner-bias-from-template`: that brief reads the curation output; this brief feeds back INTO the curation pipeline. They're orthogonal but related. Shipping order doesn't matter; both can land independently.
