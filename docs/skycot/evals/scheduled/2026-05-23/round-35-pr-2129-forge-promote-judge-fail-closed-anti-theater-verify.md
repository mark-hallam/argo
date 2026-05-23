# Round 35 — PR #2129 anti-theater verify (eval-forge-promote-fail-closed-judge-wire)

- **Date**: 2026-05-23
- **Round**: 35 (scheduled-task hourly fire)
- **Pillar/Claim**: Claim 4 (Skills are shared and learned) — fail-CLOSED judge gate on `forge promote`; supports skill-promotion safety contract.
- **PR**: [#2129](https://github.com/mark-hallam/skycot/pull/2129) `eval-forge-promote-fail-closed-judge-wire` — MERGED 2026-05-23T12:12:10Z, +483/-0, 1 file (`crates/argo-judge/tests/forge_promote_fail_closed_contract_test.rs`).

## Verdict

🟢🟢 **PASS / P2** — every substrate-contract assertion verified against `origin/main` source; the `#[ignore]`'d FINDING about every production caller passing `judge: None` is **real** — confirmed by reading `crates/argo-cli/src/handlers/forge_optimize.rs`. Anti-theater grade: gold-standard — 10 active tests pin substrate (variant set, fail-CLOSED via `ArgoResult`, `tokio::time::timeout` primitive, default rubric thresholds, SkillRefine round-trip + serde tag) PLUS 1 deliberately-failing `#[ignore]`'d test that surfaces the wire gap with a named fix-brief.

## Setup

```bash
# substrate citations from the test file
rg -n 'pub enum JudgeVerdict|pub trait Judge|pub struct Rubric|pub fn default_rubric|verdict_thresholds' \
   crates/argo-types/src/*.rs crates/argo-judge/src/*.rs

# the FINDING: every forge_optimize entry point passes judge: None
grep -n 'run_with_events_and_run_id\|judge: None\|judge: Some' \
   crates/argo-cli/src/handlers/forge_optimize.rs
```

## Observed

### Substrate-contract citations — all real

| Claim from test | Cite in `origin/main` | Verdict |
|---|---|---|
| `JudgeVerdict` has exactly 3 variants | `crates/argo-types/src/judge.rs:62-77` — `enum JudgeVerdict { Success, Partial { reason }, Failure { reason } }` | 🟢 |
| `serde(tag = "verdict", rename_all = "snake_case")` (so audit-feed filter is stable) | `crates/argo-types/src/judge.rs:63` | 🟢 |
| `pub trait Judge::adjudicate(...) -> ArgoResult<JudgeRecord>` (Result return = fail-CLOSED substrate) | `crates/argo-judge/src/judge.rs:27` | 🟢 |
| `Rubric::default_rubric()` exists at the public surface | `crates/argo-judge/src/rubric.rs:59` | 🟢 |
| `verdict_thresholds` field on `Rubric` | `crates/argo-judge/src/rubric.rs:46` | 🟢 |
| Default `success` threshold = 0.8, `partial` = 0.5 | `crates/argo-judge/src/rubric.rs:97` (`VerdictThresholds {...}` literal in `default_rubric()`) | 🟢 (pinned in test PASS 4) |

### The FINDING — `forge_optimize` callers pass `judge: None`

**Test docstring claim** (lines 449-462): *"every production caller of `forge_optimize::run_with_events_and_run_id` passes `judge: None`."*

**Verified by direct read of `crates/argo-cli/src/handlers/forge_optimize.rs`:**

| Entry point | Line | Judge arg |
|---|---|---|
| `run` (CLI default) | 19-35 → calls `run_with_events` with no judge arg | inherits `None` (run_with_events fixes judge to None) |
| `run_with_events` (gateway `POST /forge/optimize/stream`) | 42-61 → line 57-58 `None, None` | 🔴 `judge: None` |
| `run_claimed` (forge daemon) | 68-86 → line 83 `None` | 🔴 `judge: None` |

All 3 entry points reach `run_with_events_and_run_id` (signature at line 89, judge param at line 97) with `judge: None`. The fail-CLOSED gate built in PR #940 is **wired but unreachable in production today**. The FINDING is structurally honest.

### Test counts — verified

```
wc -l crates/argo-judge/tests/forge_promote_fail_closed_contract_test.rs
→ 483 lines (matches PR's +483)

grep -c '#\[test\]\|#\[tokio::test\]' on the file:
→ 11 total tests (10 active + 1 #[ignore]'d FINDINGS-ONLY at line 475)
```

## Anti-theater check

The grade is gold-standard for three reasons:

1. **No test-local seams masquerading as production.** Three stubs (`FailingJudge`, `ScriptedJudge`, `PendingJudge`) implement the **real** `argo_judge::Judge` trait. The trait is the production-truth surface; the stubs let you control timing + error shape without owning a real LLM transport. The stubs **cannot** make a verdict the trait can't return.
2. **The `#[ignore]`'d FINDINGS-ONLY test is deliberately structured to fail when un-ignored.** Line 478-481: `panic!("FINDING: forge_optimize entry points all pass judge: None ...")`. The `#[ignore = "..."]` attribute documents the brief name (`mission-forge-optimize-thread-real-judge`) — when the fix lands and threads a real `LlmJudge`, the brief author un-ignores AND inverts the assertion. The test cannot quietly turn green by being deleted; the gap leaves a load-bearing test file behind.
3. **Cross-checked against existing in-crate tests.** `crates/argo-judge/tests/` (existing) cover the rubric math + `LlmJudge::parse` paths in isolation. This file is the FIRST that pins the *fail-CLOSED contract* the forge gate depends on — the four orthogonal axes (variant exhaustiveness, Err-propagation, timeout-primitive, missing-check-reduces-to-Failure) collectively defend against the four refactors that would silently weaken the gate.

## Gap / follow-up brief candidates

**None new from this verification.** The test itself names the next-needed brief in line 467-469 docstring:

- **`mission-forge-optimize-thread-real-judge`** (P2, ~80-150 LOC) — construct `LlmJudge::with_default_rubric` against the session's resolved transport + judge model, thread it through `run_with_events_and_run_id`, gate behind `ARGO_FORGE_JUDGE=on` env var for v1.x rollout, un-ignore + invert the `findings_forge_optimize_production_callers_pass_judge_none` test once the wire is live.

This brief is **the natural next worker pickup** — the regression contract is literally pre-baked in #2129's ignored test; the worker un-ignores + flips the assertion + wires the judge through 3 callsites in `argo-cli`.

## Notes

- 13th green mission-claim regression contract on main this week (claim 1 round 17; claim 6 strengthener round 21; claim 6 MVP rows 1/2/5/4 rounds 22/23/24/27; claim 2 substrate round 26; claim 4 rows 7 + 4 rounds 28/29; claims 3+5+6 + cu-macos rounds 31/32; #2125 + #2126 + #2121 + #2116 rounds 33/34; **this PR's 10 fail-CLOSED substrate tests round 35**).
- Anti-theater verification proceeded by source-code inspection alone — concurrent conductor activity meant I did NOT attempt `cargo test -p argo-judge` (round-33 documented the worker-tree-reset flake under concurrent push).
- No code touched. Charter compliance: read-only across `crates/`.
