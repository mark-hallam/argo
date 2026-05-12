# Mission load-bearing wire — planner-bias-from-template wiring

**Owner**: claude. **Depends on**: existing `argo-runtime::planner_templates::PlanTemplate` + `TemplatePlanner` (with `with_learned_templates()` constructor — already wired); `argo-runtime::dreams_plan_templates::LearnedPlanTemplate` (Dreams curation output); `mission-llm-call-dedup-cache` and `mission-judge-disagreement-eval` are sibling self-improvement-infra briefs (independent).
**Authored against**: origin/main `f1b6b0d1` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-planner-bias-from-template`.

> SOURCE: MISSION_BACKLOG.md Section 5 Claim 7-9 #195 ("Build planner-bias-from-template"). **Tier M (self-improvement).** Today: `TemplatePlanner` exists with `with_learned_templates(Vec<PlanTemplate>)`, and `dreams_plan_templates` produces `LearnedPlanTemplate` from Dreams curation. BUT: the runtime dispatch loop does NOT consult learned templates before generating a fresh plan — every planner call is cold. This brief lands the BIAS path: when the planner has a learned-template that matches the current task signature with high enough trust-grade, it prefers that template over generating from scratch. The platform's "skills get better over time" promise depends on this wire.

## Scope

EXTEND `argo-runtime` to consult learned templates in the planner dispatch path:

### New module: `crates/argo-runtime/src/planner_bias.rs` (NEW, ≤ 220 LOC)

- `pub struct PlannerBias` — wraps a `TemplatePlanner` + a `LearnedPlanTemplateSource` (existing trait); on `plan(task)`:
  1. Compute `task_signature(task)` — stable hash of (vertical_id, skill_name, normalised-input-shape). Use `argo-types::TaskSignature` if it exists; else introduce in this brief (≤ 30 LOC).
  2. Look up matching templates from `LearnedPlanTemplateSource` with signature + min-trust-grade ≥ threshold (default: `TrustGrade::Bronze` — configurable via `[planner].bias_min_trust_grade`).
  3. If a match exists, return it as the plan (with a `PlanProvenance::FromLearnedTemplate { template_id, trust_grade, signature }` annotation).
  4. If no match, fall through to the existing `TemplatePlanner::plan(task)` (cold-generate path).
- `pub trait LearnedPlanTemplateSource: Send + Sync` — `fn lookup(&self, sig: &TaskSignature, min_grade: TrustGrade) -> Vec<LearnedPlanTemplate>`. Two impls:
  - `InMemoryTemplateSource` (test fixture)
  - `StoreTemplateSource` (consults `argo-store::dreams_plan_templates` via `MemoryNs`-style query)
- 8 unit tests covering: cold-miss → fallthrough, single-match hit, multiple-matches → trust-grade-ordered selection, below-threshold-skip, signature-stability across whitespace + ordering, bias-disabled config flag, provenance annotation round-trip, source-error-fallthrough (lookup err ⇒ cold-generate, never crash).

### Config wire-up

EXTEND `crates/argo-config/src/sections.rs` (≤ 25 LOC delta):

```toml
[planner.bias]
enabled = true                       # default: true
min_trust_grade = "Bronze"           # Bronze | Silver | Gold | Platinum
prefer_per_vertical = true           # if multiple templates match, prefer the per-vertical one
require_recent_within_days = 30      # 0 = ignore recency
```

### Dispatch-loop integration

EXTEND `crates/argo-runtime/src/lib.rs` (≤ 15 LOC) — replace the planner-call site with `PlannerBias::plan(task)` instead of `TemplatePlanner::plan(task)`. The `AgentDeps` injection point gets a `Arc<dyn LearnedPlanTemplateSource>` field; default impl returns empty (no bias).

Also EXTEND the `worker_actions_emit` dispatch (the path the worker takes to generate next actions) to annotate the audit-chain row with the `PlanProvenance` so operators see whether a worker used a learned template or cold-generated.

### Provenance + reporting

**NEW** `argo-types::PlanProvenance` enum (≤ 25 LOC) — define the enum from scratch with three variants: `ColdGenerated`, `BuiltIn`, `FromLearnedTemplate { template_id, trust_grade, signature }`. Earlier draft of this brief incorrectly claimed the enum already existed; verified against origin/main 2026-05-12 — `PlanProvenance` does not appear anywhere in the workspace.

**NEW** migration: this brief uses **slot 68** per the header pin (slots 63-70 reserved; this brief owns 68). There is NO `plan_history` table on origin/main 2026-05-12; verified via `rg -n 'CREATE TABLE plan_history\|plan_history (' crates/argo-store/migrations/`. Two options for the schema, pick the cleaner one at claim time:

  (a) **NEW** `plan_provenance` table — `(id INTEGER PRIMARY KEY, session_id BLOB, worker_id BLOB, created_at INTEGER NOT NULL, provenance_json TEXT NOT NULL)` — keeps provenance out of `audit_record` payload (parallel observability table).

  (b) Stamp the provenance into `audit_record.payload_json` directly under a top-level `plan_provenance` key. No new table; reuses the existing audit chain. Lower complexity, but couples provenance reads to audit-row scans.

The brief preference is (b) — pure additive stamping inside the audit chain — unless the operator wants a dedicated query surface, in which case (a). Document the choice + reasoning in the PR body.

If (a) is chosen, the migration goes to slot 68 (`68__plan_provenance.sql`). If (b), no migration needed; just additive payload stamping in `worker_actions_emit` audit-chain write.

## Acceptance criteria

- [ ] `cargo test -p argo-runtime planner_bias` passes all 8 unit tests
- [ ] `argo run --vertical <id> --skill <name>` with a learned-template present produces a plan annotated `FromLearnedTemplate { ... }` in the audit-chain row
- [ ] `argo plan-template list --json` includes a `trust_grade` field (re-uses existing field — verify it's already there)
- [ ] Toggling `[planner.bias].enabled = false` in `argo.toml` makes the planner ALWAYS cold-generate (per-config trap test)
- [ ] `[planner.bias].min_trust_grade = "Gold"` filters out Bronze/Silver templates from the bias path (verified by integration smoke)
- [ ] Below-threshold AND no-match → cold-generate fallthrough; never crashes the worker (provenance annotates `ColdGenerated`)
- [ ] Migration `vNN.rs` adds `plan_provenance` column idempotently (verify with migration-walk test)
- [ ] No file > 500 LOC; per-file delta ≤ 220 (planner_bias.rs) / ≤ 25 (config) / ≤ 15 (lib.rs + types) / ≤ 70 (migration)
- [ ] BUILD_STATUS.md updated with PR number

## Files you may touch

- `crates/argo-runtime/src/planner_bias.rs` (NEW, ≤ 220 LOC)
- `crates/argo-runtime/src/planner_bias_tests.rs` (NEW, ≤ 200 LOC, included via `#[path]`)
- `crates/argo-runtime/src/lib.rs` (EXTEND, ≤ 15 LOC delta — swap planner call site)
- `crates/argo-config/src/sections.rs` (EXTEND, ≤ 25 LOC delta — `[planner.bias]` section)
- `crates/argo-config/src/defaults.rs` (EXTEND, ≤ 10 LOC delta)
- `crates/argo-types/src/plan_provenance.rs` (EXTEND OR NEW, ≤ 25 LOC — add `FromLearnedTemplate` variant)
- `crates/argo-store/src/migrations/vNN.rs` (NEW; verify slot — v59 at brief author time, but `mission-gateway-request-audit-log` + `mission-audit-chain-watchdog` are parallel reservation-only briefs in this range; check `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -3` at claim time)
- `crates/argo-store/migrations/<NN>__plan_provenance.sql` (NEW; verify slot ≥ 63)
- `crates/argo-store/src/migrations/mod.rs` (EXTEND only — register new migration module)
- `crates/argo-cli/src/handlers/worker_actions_emit.rs` (EXTEND, ≤ 30 LOC delta — annotate audit-chain row)

## Files you must NOT touch

- `crates/argo-runtime/src/planner.rs` (the cold-generate path stays; bias wraps it)
- `crates/argo-runtime/src/planner_templates.rs` (existing TemplatePlanner stays; bias wraps it)
- `crates/argo-runtime/src/dreams_plan_templates.rs` (LearnedPlanTemplate stays; bias consumes it)
- Any vertical manifest

## Branch + PR

- Branch: `claude-mission-planner-bias-from-template`
- PR title: `runtime: planner-bias-from-learned-template (TrustGrade-gated, provenance-annotated, configurable) (mission-planner-bias-from-template)`

## Notes for the implementer

- The `PlannerBias` wrapper composes — it does NOT replace `TemplatePlanner`. The fall-through case keeps `TemplatePlanner` behaviour identical; this means tests that exercise the cold-generate path continue to work without modification.
- `TaskSignature` is a stable hash. The brief assumes (vertical_id, skill_name, normalised-input-shape) is the right tuple; if input contains operator-specific PII, that should NOT be in the signature (use field-presence + field-type, not field-values). Document this in the rustdoc.
- `min_trust_grade` defaults to `Bronze` — relatively permissive — because Dreams curation is the gatekeeper for which templates earn a TrustGrade at all. The operator can tighten to `Gold` via config.
- `prefer_per_vertical` — when multiple templates match, the per-vertical one (template scoped to this exact vertical_id) wins over the all-verticals fallback. This matches the operator_risk_overrides scope-precedence pattern (#1361).
- `require_recent_within_days` — stale templates shouldn't bias toward outdated patterns. If a learned template hasn't been refreshed by a Dreams run within N days, the bias falls through to cold-generate. Default 30; 0 = ignore recency.
- Audit-chain annotation: `PlanProvenance::FromLearnedTemplate { template_id, trust_grade, signature }` makes the bias VISIBLE in audit. An operator scanning for "why did the worker pick this plan" can read it. This is the load-bearing observability concession — without it, the bias is invisible and debug becomes a black box.
- `bias_min_trust_grade = "Gold"` is the "safety-conservative" config — only platinum-curated templates bias the planner. Suitable for production with stable Dreams curation. `"Bronze"` is the experimentation default — try bias eagerly + observe.
- `StoreTemplateSource` should NOT cache lookups in v1 — keep it simple. A cache layer is a v2 follow-up if benchmark shows the lookup is hot.
- DO NOT introduce a "bias-confidence-score" that mixes trust-grade + recency + match-quality into a single float. Operators tune three independent dials (trust_grade / recency / per-vertical) — mixing them obscures the trade-offs.
- DO NOT add an "automatic-template-promotion-from-cold-generate" path. Promotion is Dreams' job; bias only consumes promoted templates.
- The dispatch-loop wire is a SWAP, not an add — the existing `planner.plan(task)` call site becomes `planner_bias.plan(task)` with `planner_bias` constructed from the existing planner + the new source. Operator visibility into the swap comes from `PlanProvenance` annotation.
- Migration coordination: at claim time, run `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -3` AND `ls crates/argo-store/migrations/*.sql | sort -V | tail -3` and pick the next free pair. As of brief authoring (2026-05-12), v58 + 62 just shipped via `mission-operator-specific-risk-overrides` (#1361); this brief expects v59 + 63 BUT `mission-gateway-request-audit-log` and `mission-audit-chain-watchdog` are parallel reservation-only briefs that may have claimed first.
