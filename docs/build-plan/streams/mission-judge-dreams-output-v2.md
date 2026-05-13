# mission-judge-dreams-output-v2 — pre-approval LLM judge on Dreams output (rewrite)

**Owner**: claude. **Depends on**: `argo-judge::LlmJudge::adjudicate` shipped; argo-skill / argo-runtime / argo-memory Dreams surfaces shipped.
**Authored against**: origin/main `6676b391` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-judge-dreams-output-v2`.

> **REWRITE 2026-05-13 by conductor (CEO-authorized Cluster D #1).** V1 (`mission-judge-dreams-output`) targeted nonexistent `argo-dreams` crate + relied on score-threshold variants that don't exist in `argo-judge`. V2 uses **existing** `JudgeVerdict::{Success, Partial, Failure}` mapping → Accept/Borderline/Reject, wired across the 3 real Dreams crates.

## Scope

The LLM judge (`argo-judge::LlmJudge::adjudicate`) is shipped but not invoked on Dreams output. Today operator approval is the first quality check on skill drafts / plan templates / memory summaries — too late, too noisy. This brief wires the judge as a pre-approval gate so operators only see Accept/Borderline drafts; Reject drafts get logged + discarded silently.

Verdict mapping (uses existing `JudgeVerdict` variants — no new types):
- `Success` → `Accept(item)` → pass through to approval queue
- `Partial` → `Borderline { item, evidence }` → pass through with tag
- `Failure` → `Reject { reason, evidence }` → log + discard

Wire one judge call site per Dreams surface (3 surfaces, 3 call sites):

1. **`argo-skill::dreams_distillation`** — gate skill drafts before approval-queue insert. Find the call site via `rg approval_queue\|enqueue_skill_proposal crates/argo-skill/src/`.
2. **`argo-runtime::dreams_plan_templates`** — gate plan templates before promotion to TrustGrade::Promoted candidacy.
3. **`argo-memory::dreams_backend`** — gate memory summaries before they reach the operator review surface.

## Parallelism

- Manager-worker allowed: no
- Max children: 0; Max depth: 1

## Files you may touch

- `crates/argo-skill/src/dreams_distillation.rs` — narrowly scoped: add judge call before approval-queue insert. ≤40 modified lines.
- `crates/argo-runtime/src/dreams_plan_templates.rs` — same shape, narrowly scoped: ≤40 modified lines.
- `crates/argo-memory/src/dreams_backend.rs` — same shape, narrowly scoped: ≤40 modified lines.
- `<NEW: crates/argo-skill/src/dreams_judge_gate.rs>` — shared helper: `JudgeGate<T>` enum + `gate_with_verdict<T>(judge, subject, item) -> ArgoResult<JudgeGate<T>>`. ≤150 LOC.
- Per-crate Cargo.toml additions: each Dreams host crate adds `argo-judge` dep if not already present.
- Tests alongside each call site (sibling `_tests.rs`) — 3 tests per crate covering the 3 verdict branches.

## Files you must NOT touch

- `crates/argo-judge/**` — `Judge` trait + `JudgeVerdict` enum locked.
- `crates/argo-cli/src/handlers/skill*.rs` — skill-creation judge gate (#68) is separate.
- The operator approval-queue UI/CLI — output shape unchanged; only what reaches the queue changes.

## Acceptance criteria

- [ ] `JudgeGate<T>` enum + `gate_with_verdict` helper in shared module.
- [ ] All 3 Dreams call sites consult the judge before approval-queue insert.
- [ ] Reject branch: emits `tracing::warn!(target = "dreams_judge", ?reason, ?evidence)` + writes to `AuditSink`.
- [ ] Borderline branch: passes through but emits a structured tag the operator UI can render.
- [ ] 9 unit tests (3 per crate × 3 verdict branches) + 1 integration smoke per Dreams surface.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-judge-dreams-output-v2`
- PR title: `dreams: judge gate on Dreams output across 3 host crates (mission-judge-dreams-output-v2)`

## Notes for the implementer

- Per CLAUDE.md "single-binary": share the gate helper rather than duplicating per crate; it lives in argo-skill (or argo-judge if it grows) and the other crates `use argo_skill::dreams_judge_gate::*`.
- Rubric: `min_severity = Warn` for Borderline boundary; concrete score thresholds are NOT in scope (use the existing verdict variants as the boundary).
- This brief is Mission Claim 4 (self-improvement) plumbing — without the judge, operator review noise stays high.
