# Mission load-bearing wire — self-evaluation honesty test for skills

**Owner**: claude. **Depends on**: `argo-eval` crate SHIPPED (verified at `crates/argo-eval/src/`: runner, task, compare modules); `argo-skill::quality::QualityScore` SHIPPED.
**Authored against**: origin/main `9da70ced` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-eval-self-honesty-test`.

> SOURCE: MISSION_BACKLOG.md Section 6 #210 ("Build self-evaluation honesty test"). Today skills self-report success rates via their own outcome tracking, but there's no independent test that catches a skill **systematically over-reporting** its own quality. Mission Claim 4 ("skills shared and learned") trust depends on honest self-reports — without an honesty test, a skill that gradually drifts toward fake-success answers (to look good) can game its trust grade.

## Scope

Add `argo-eval::honesty` module with `run_honesty_test(skill_id, store, cfg) -> HonestyResult`:

```rust
pub struct HonestyConfig {
    pub sample_size: usize,            // default 20
    pub min_history_count: usize,      // default 50; skip if insufficient
    pub disagreement_threshold: f32,   // default 0.30 (30%)
}

pub enum HonestyResult {
    NotEnoughHistory,
    Honest { self_reported: f32, observed: f32, delta: f32 },
    Suspicious { self_reported: f32, observed: f32, delta: f32, sample_size: usize },
}
```

Method:
1. Sample N recent runs of the skill from `audit_record` (where audit captured the tool_result + skill self-report).
2. For each sample: extract the skill's `self_reported_success` field from the audit payload (skills emit this in their tool_result envelope).
3. Independently classify each run as actually-successful by running it through a re-judge step (calls `argo-judge::adjudicate` with the original input + output — already shipped).
4. Compute disagreement rate: `(|self_reported - judge_observed|) / sample_size`.
5. If disagreement ≥ threshold AND sample_size ≥ min_history_count: emit `Suspicious`. Else `Honest`.

NEW `argo eval skill-honesty <skill_id> [--sample <N>]` CLI prints the result + sample-level breakdown for operator triage.

The honesty test is **diagnostic**, not enforcement. Trust-grade demotion based on suspicion is a downstream brief.

This is NOT in scope: changing skill self-report shape; adjusting trust grades from honesty findings; cross-tenant honesty correlation; per-run honesty (only batch-aggregate).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-eval/src/honesty.rs` (NEW, ≤ 300 LOC) — `HonestyConfig`, `HonestyResult`, `run_honesty_test`, narrow `AuditSampler` trait + impl.
- `crates/argo-eval/src/honesty_tests.rs` (NEW, sibling) — 8 unit tests: not-enough-history; perfect-agreement-honest; high-disagreement-suspicious; threshold-boundary; empty-audit-NotEnoughHistory; missing-self-reported-skipped; sample-size-respected; judge-failure-falls-through.
- `crates/argo-eval/src/lib.rs` — narrowly scoped: ONE `pub mod honesty;` line + re-export.
- `crates/argo-cli/src/handlers/eval_skill_honesty.rs` (NEW, ≤ 150 LOC) — `argo eval skill-honesty` CLI handler.
- `crates/argo-cli/src/cli_eval_cmds.rs` (verify path in first 5min) — narrowly scoped: ADD `skill-honesty` arm.
- `crates/argo-cli/tests/eval_skill_honesty_smoke.rs` (NEW, ≤ 120 LOC) — integration smoke: seed audit with 25 runs (20 honest + 5 lying), assert Suspicious returned with the right delta.

## Files you must NOT touch

- `crates/argo-judge/**` — judge surface is upstream; reuse only
- `crates/argo-skill/src/quality.rs` — quality is downstream consumer; orthogonal
- `crates/argo-audit/**` — audit chain is upstream
- Any non-eval/non-cli crate
- `crates/argo-store/migrations/**` — no schema change (reuses existing audit_record)

## Acceptance criteria

- [ ] `HonestyConfig` defaults: sample_size=20, min_history_count=50, disagreement_threshold=0.30
- [ ] `run_honesty_test` returns `NotEnoughHistory` when audit count < min_history_count
- [ ] Returns `Honest` when |self_reported - observed| < threshold
- [ ] Returns `Suspicious` when ≥ threshold AND sufficient history
- [ ] Judge step reuses `argo-judge::adjudicate` (no custom logic)
- [ ] Per-run breakdown in CLI output for operator triage
- [ ] 8 unit tests + 1 integration smoke (20 honest + 5 lying → Suspicious)
- [ ] No new public API outside the listed exports
- [ ] No DB migration (uses existing audit_record)
- [ ] `cargo build -p argo-eval -p argo-cli` clean
- [ ] `cargo clippy -p argo-eval -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-eval -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-eval-self-honesty-test`
- PR title: `eval: skill self-evaluation honesty test (mission-eval-self-honesty-test)`

## Notes for the implementer

- The 30% disagreement threshold is generous on purpose — small audit samples are noisy, and we want false-positive Suspicious rates near zero. Operators can tighten via config.
- Reuse `argo-judge::adjudicate` for the independent classification — do NOT invent a new judge here.
- The "lying" test case in the smoke test should have ALL 5 lies be high-confidence successes that the judge classifies as failures. Real-world drift is more subtle — but the test just proves the math, not the production tuning.
- If the audit payload schema doesn't already carry `self_reported_success`, BLOCK with a precursor brief `mission-skill-self-report-field` — do NOT expand audit schema in this brief.
