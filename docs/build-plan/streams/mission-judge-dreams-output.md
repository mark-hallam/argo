# Mission load-bearing wire — judge gate on Dreams output

**Owner**: claude. **Depends on**: `argo-judge` crate SHIPPED (`crates/argo-judge/src/judge.rs` exposes `LlmJudge::adjudicate`); Dreams non-destructive output behind operator approval SHIPPED (folded into #186); Dreams skill-distillation SHIPPED.
**Authored against**: origin/main `09673df7` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-judge-dreams-output`.

> SOURCE: MISSION_BACKLOG.md Section 6 #193 ("Wire judge into Dreams output validation"). Mission claim 4 (self-improvement) ships Dreams + skill-distillation + plan-templates + operator-approval gating today. The remaining gap: the judge already gates skill *creation* (#68 SHIPPED), but Dreams *output* (skill bodies, plan templates, memory summaries) isn't run through the judge before reaching the operator-approval queue. That means operator approval is the first quality check on Dreams output — too late and too noisy. Wire the judge as a pre-approval gate so only `Accept` or `Borderline` Dreams output reaches operator review; `Reject` output gets logged + discarded silently.

## Scope

Add a NEW `dreams_judge` module to `argo-dreams` (verify crate exists via `ls crates/argo-dreams/src/` in first 5min — if the crate name differs, mirror the actual canonical name):

```rust
// crates/argo-dreams/src/dreams_judge.rs
pub struct DreamsOutputJudge<J: Judge> { judge: J, rubric: DreamsRubric }
pub struct DreamsRubric { pub min_score: u8, pub min_evidence_quality: EvidenceQuality }
pub enum JudgeGate<T> {
    Accept(T),                          // verdict ≥ min_score → pass through to approval queue
    Borderline { item: T, evidence: JudgeEvidence },  // tag + pass through but flag
    Reject { reason: String, evidence: JudgeEvidence }, // discard + audit-log
}
impl<J: Judge> DreamsOutputJudge<J> {
    pub fn new(judge: J, rubric: DreamsRubric) -> Self;
    pub async fn gate_skill_distillation(&self, draft: SkillDraft) -> ArgoResult<JudgeGate<SkillDraft>>;
    pub async fn gate_plan_template(&self, draft: PlanTemplateDraft) -> ArgoResult<JudgeGate<PlanTemplateDraft>>;
    pub async fn gate_memory_summary(&self, draft: MemorySummaryDraft) -> ArgoResult<JudgeGate<MemorySummaryDraft>>;
}
```

Each `gate_*` method:
1. Builds a judge subject from the draft (reuse `JudgeSubject::SkillCreation` for skill drafts; add `JudgeSubject::PlanTemplate` and `JudgeSubject::MemorySummary` if missing — verify with `grep enum JudgeSubject` first).
2. Calls `judge.adjudicate(subject, rubric)` → `JudgeVerdict`.
3. Maps `Verdict::Accept(score)` with `score ≥ rubric.min_score` → `Accept`; `score < min_score && score >= min_score-2` → `Borderline`; else → `Reject`.
4. On `Reject`, emits a `tracing::warn!(target = "dreams_judge", ?reason, ?evidence)` and audits via existing `AuditSink::log_event` (reuse — don't reinvent the chain).

The Dreams output pipeline (the place that calls Claude Dreams API → produces drafts → enqueues operator approval) must be wired to invoke the gate per draft kind, before the existing approval-queue enqueue. Locate the call site via `rg -n 'enqueue.*approval\|approval_queue\|enqueue_skill_proposal' crates/argo-dreams/src/`. Wire callsites narrowly — one match per draft kind.

This is NOT in scope: judge model selection (operator config); rubric tuning per draft kind beyond `min_score`; multi-round judge (single-shot only); promoting `Borderline` UX into operator inbox (operator-approval surface change is separate); changing the judge crate's `Judge` trait.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-dreams/src/dreams_judge.rs` (NEW, ≤ 280 LOC) — module + `DreamsOutputJudge` + `JudgeGate` enum
- `crates/argo-dreams/src/dreams_judge_tests.rs` (NEW, sibling, ≤ 250 LOC) — 7 unit tests w/ a `MockJudge` stub
- `crates/argo-dreams/src/lib.rs` — narrowly scoped: ONE `pub mod dreams_judge;` line + re-exports
- `crates/argo-dreams/src/<pipeline-file>.rs` — narrowly scoped: ADD `DreamsOutputJudge::new(...)` instantiation + 3 `gate_*` calls before the existing approval-enqueue path; verified file path in first 5min via `rg -n approval_queue crates/argo-dreams/src/`
- `crates/argo-dreams/Cargo.toml` — narrowly scoped: ADD `argo-judge` dep (likely already present; verify)
- `crates/argo-dreams/tests/dreams_judge_smoke.rs` (NEW, ≤ 200 LOC) — integration smoke: deterministic mock judge, walks a skill-draft + plan-template + memory-summary through the pipeline, asserts Reject is dropped + Accept reaches the approval-enqueue mock

## Files you must NOT touch

- `crates/argo-judge/**` — Judge trait stable; reuse
- `crates/argo-cli/src/handlers/skill*.rs` — skill-creation judge gate (#68) is separate path
- `crates/argo-store/migrations/**` — no schema change
- The operator-approval inbox crate / web surface — Borderline UX surfacing is separate

## Acceptance criteria

- [ ] `DreamsOutputJudge::new` constructs with rubric defaults `min_score = 6`, `min_evidence_quality = EvidenceQuality::Adequate`
- [ ] `gate_skill_distillation` returns `Accept` iff verdict score ≥ 6, `Borderline` for 4–5, `Reject` for ≤ 3
- [ ] `gate_plan_template` and `gate_memory_summary` follow same mapping
- [ ] `Reject` path emits `tracing::warn!` + audit event; drops draft (not enqueued)
- [ ] `Borderline` path flags + enqueues (still reaches operator)
- [ ] Existing approval-enqueue call sites are wrapped — `rg "enqueue_skill_proposal\|approval_queue_push" crates/argo-dreams/src/` shows the gate is called before each
- [ ] 7 unit tests: accept-path, borderline-path-still-enqueues, reject-path-discards, reject-emits-audit, score-boundary-min, score-boundary-borderline-floor, judge-error-surfaces (don't swallow judge failure)
- [ ] 1 integration smoke: full skill-draft + plan-template + memory-summary walk
- [ ] No new `unsafe` / `unwrap` / `expect` outside `#[cfg(test)]`
- [ ] `cargo clippy -p argo-dreams --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-dreams` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-judge-dreams-output`
- PR title: `dreams: judge gate on Dreams output before operator approval (mission-judge-dreams-output)`

## Notes for the implementer

- First step is verification: confirm crate name `argo-dreams` exists, locate the approval-enqueue callsites, confirm `JudgeSubject::PlanTemplate` and `JudgeSubject::MemorySummary` variants. If those variants don't exist in `argo-types`, add them under HARD RULE 5 exhaustive-match allowance — extending `JudgeSubject` is narrow + necessary.
- The `Reject` audit event reuses the existing `AuditKind::ToolResult` or analogous structured kind. Do NOT add a new `AuditKind` variant in this PR — that's a separate brief.
- Rubric `min_score = 6` is the operator's pragmatic floor (matches #68 skill-creation gate). Don't make it configurable in this PR — operator-config-as-rubric is a separate brief.
- `MockJudge` in tests: implement `impl Judge for MockJudge` returning verdicts from a `VecDeque<JudgeVerdict>` queue — same pattern as judge-disagreement-eval suite.
- The Dreams pipeline likely lives in `crates/argo-dreams/src/{pipeline,output,enqueue}.rs` — locate by grep, verify, narrow the may-touch to the actual file.
