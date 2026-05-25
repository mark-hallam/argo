# Round 65 · PR #2295 PRE-MERGE verify — `Track P — P-02 — enforce truth boundary at turn exit` 🟡 PARTIAL / BLOCKED

- **Date**: 2026-05-25 (Melbourne 2026-05-26 00:47 AEST)
- **Round**: 65
- **Pillar / Claim**: Track P (v1.0 demo headline — operator-locked 2026-05-24); P-02 is the runtime gate that turns the P-01 capability into actual enforced refusals at the final assistant-response exit.
- **Brief**: `docs/build-plan/streams/pitch-truth-boundary-runtime-enforcement.md`

## Verdict

🟡 **PARTIAL / BLOCKED** — P1 rebase required + P1 LOC-budget breach. Do **not** merge until both resolved.

## Setup

- PR head: `e624dea2e3fdd8c88f4cbf07ce55c98bb8b42931`
- Mergeable: `CONFLICTING` (gh metadata) — confirmed via local rebase.
- Files: 12 (+1062/-6) — production wires P-01 truth-boundary eval into the final assistant-response exit path, adds refusal-template registry, plumbs AgentDeps fields for active policy/channel/templates, adds claim audit kinds.
- Worktree: `/tmp/argo-r65-pr2295-wt` (PR head) — rebased against `origin/main`.

## Observed

### Rebase attempt

```
$ git rebase origin/main
Auto-merging crates/argo-audit/src/event.rs
Auto-merging crates/argo-audit/src/event_kind.rs
Auto-merging crates/argo-audit/src/event_tests.rs
CONFLICT (content): Merge conflict in crates/argo-audit/src/event_tests.rs
```

### Conflict shape — single file, single hunk

`crates/argo-audit/src/event_tests.rs` line 13-32, `all_kind_count_tracks_declared_variants`:

- **Main side**: `assert_eq!(AuditKind::all().len(), 73);`
- **PR side**: `assert_eq!(AuditKind::all().len(), 75);` + a new `claim_truth_boundary_kinds_round_trip_and_listed` test block that round-trips the three new variants (`ClaimAllowed`, `ClaimBlocked`, `ClaimEscalated`).

The conflict is mechanical: PR was authored against a 72-variant baseline, main has since added `WorkerKpiRolled` (taking it to 73). After rebase, the correct count should be **76** (73 main + 3 PR claim variants) and the new claim-roundtrip test block must be retained alongside the existing main tests.

### Variant-diff confirmation (`event_kind.rs`)

```
$ diff origin/main:crates/argo-audit/src/event_kind.rs PR-head
+ ClaimAllowed,    // Truth-boundary gate allowed a candidate utterance to be delivered.
+ ClaimBlocked,    // Truth-boundary gate blocked a candidate utterance and delivered a refusal.
+ ClaimEscalated,  // Truth-boundary gate queued human approval before delivering an answer.
- WorkerKpiRolled, // KPI-rollup scheduler persisted per-worker KPI JSON.  (still present on main, missing from PR head)
```

PR head is missing `WorkerKpiRolled` because it wasn't in main when the PR was authored.

### LOC-budget breach (carry from round 60 + round 61)

```
$ wc -l crates/argo-audit/src/event_tests.rs
  499 origin/main
  512 PR-head      ← OVER the 500-LOC per-file ceiling
```

The brief carries the same 13-LOC over-budget that round 60 + round 61 already flagged. The `parallel-pub-types-enforce` + `loc-budget` CI gates would reject this on workspace-clippy; admin-merge `--skip-clippy` bypass would land the breach unfixed.

## Anti-theater check

N/A — couldn't reach the cargo test stage. The conflict is structural, the LOC breach is structural; both have to clear before "is the new gate actually wired correctly?" becomes a meaningful question.

## Gap / follow-up brief candidates

Two outstanding briefs need to land **before** PR #2295 can merge:

1. **🔴 P1 (NEW carry-over) `fix-pr-2295-rebase-and-bump-variant-count-to-76`** — mechanical 1-file rebase: take PR head, rebase, resolve `event_tests.rs` by keeping main's `WorkerKpiRolled` block + adding PR's 3 Claim variants + bumping `len() == 76` + keeping the new `claim_truth_boundary_kinds_round_trip_and_listed` test. ~10 min for any worker.
2. **🔴 P1 (carry from round 60 + r61 + r64) `fix-split-event-tests-rs-loc-budget-breach`** — `event_tests.rs` at 512 LOC violates the 500-line ceiling. Extract a sibling test module (e.g. `event_tests_claim_kinds.rs` for the 3 truth-boundary tests, included via `#[path]` from `event.rs` like the existing `event_tests.rs`) to bring both under 500. This unblocks #2295 + any future variant-add PRs.

**Order**: split first (item 2), then rebase (item 1) on the post-split main. Otherwise the rebase has to re-do the split anyway.

## Outstanding signals (informational)

- `event_kind.rs` LOC: 224 main / 228 PR head — well within budget.
- New code is well-isolated: `turn_helpers_truth_boundary.rs` (+439 NEW), `turn_helpers_truth_boundary_tests.rs` (+478 NEW), `refusal_registry.rs` (+52 NEW). No surprise blast-radius touches.
- PR body's verification commands look complete (cargo test for `argo-agent-runner` truth_boundary, clippy across 4 crates, migration-slot check, brief-freshness check, codex-pre-push). If round-66 (post-rebase + post-split) re-verifies and the PR-body cargo block still passes, that'll be a strong PRE-MERGE PASS signal.

## Cost note

Wall: ~6 min. Tokens: ~6K. Cheap because we caught the conflict before sinking cargo build time.
