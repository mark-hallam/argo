# fix-pr-2295-rebase-and-bump-variant-count

**🔴 P0 BLOCKED-on-dep rebase brief — Track P P-02 (operator-locked v1.0 demo headline)**

Authored against: origin/main `0c48732b8` on 2026-05-25
Validate before claim: bash scripts/check-brief-freshness.sh fix-pr-2295-rebase-and-bump-variant-count

## Dependency

**BLOCKED on `fix-split-event-tests-rs-loc-budget-breach` MERGED.** That brief reduces `crates/argo-audit/src/event_tests.rs` from 499 LOC to ≤350 by extracting variant-cluster sibling files. PR #2295 currently breaches HARD RULE 6 (LOC budget) at 512 LOC post-its-own-additions — until the split lands, rebasing #2295 will re-hit the budget breach. **Do NOT claim this brief until the split PR MERGES.**

## Why this exists

PR #2295 is Track P P-02 — the runtime gate that enforces TruthBoundaryPolicy at agent-turn EXIT, the operator-locked v1.0 demo headline. The PR adds 3 new `AuditKind` variants (`ClaimAllowed`/`ClaimBlocked`/`ClaimEscalated`) + truth-boundary runtime enforcement + 11 runner tests. It's been DIRTY since evaluator round-60 (3 rounds ago — r60/r61/r64/r65 all flagged).

Two structural blockers per evaluator round-65:
- **(a) Content conflict** in `event_tests.rs` — PR was authored at 72-variant baseline; main has since added `WorkerKpiRolled` (→ 73); PR adds 3 new Claim variants; post-rebase count should be 76.
- **(b) LOC budget** — `event_tests.rs` at 512 on PR head vs 500 ceiling (carry through r60/r61/r64).

## What this brief does

ONCE `fix-split-event-tests-rs-loc-budget-breach` MERGES (post-#2295-rebase will fit under 500 LOC ceiling), execute:

1. `git fetch origin main`
2. `git checkout claude-mission-track-p-p-02-truth-boundary-turn-exit` (verify actual branch name via `gh pr view 2295 --json headRefName`)
3. `git rebase origin/main`
4. Resolve `event_tests.rs` conflict — accept the split-extracted sibling structure on main + relocate the 3 new Claim variant tests into the appropriate sibling file (per `fix-split-event-tests-rs-loc-budget-breach`'s variant-cluster decision)
5. Bump `AuditKind` variant count to 76 (or whatever current main count is + 3) — single line in `event_kind.rs` + matching label-table entries + `AuditKind::all()` listing
6. Re-verify Track P P-02 runtime enforcement tests still pass (`cargo test -p argo-agent-runner truth_boundary_turn_exit`)
7. `scripts/workspace-clippy-fast-fail.sh` clean
8. Force-push to existing branch (preserve PR #2295 review history)
9. PR #2295 flips MERGEABLE/CLEAN

## Critical: this PR is the operator-locked v1.0 demo headline

Per CLAUDE.md "🚨 OPERATOR AMENDMENT 2026-05-24 — TRACK P (PITCH AGENT) IS IN v1.0 MVP SCOPE": Track P P-02 is required for the v1.0 demo MP4. Do NOT let this PR rot. Once `fix-split-event-tests-rs` MERGES, claim and ship this rebase immediately.

## Acceptance criteria

1. Dependency `fix-split-event-tests-rs-loc-budget-breach` is MERGED on origin/main BEFORE claim
2. Rebase completes with manual conflict resolution
3. AuditKind variant count + label table + AuditKind::all() listing all show count 76 (or current+3)
4. `cargo test -p argo-agent-runner --test truth_boundary_turn_exit` passes
5. `cargo test -p argo-audit` passes (variant count assertions)
6. `scripts/workspace-clippy-fast-fail.sh` clean
7. PR #2295 MERGEABLE/CLEAN

## May-touch

- `crates/argo-audit/src/event.rs`, `event_kind.rs`
- `crates/argo-audit/src/event_tests*.rs` (any sibling files created by the split)
- `crates/argo-agent-runner/src/turn_helpers*.rs`
- `crates/argo-agent-runner/src/dispatch.rs`
- `crates/argo-agent-runner/tests/truth_boundary_turn_exit.rs`
- The same surfaces the original PR #2295 touched (do NOT widen)

## Must NOT touch

- BUILD_STATUS.md
- PR #2295 description
- Other crates not in original PR scope

## Verification

```bash
git fetch origin main
git checkout claude-mission-track-p-p-02-truth-boundary-turn-exit
git rebase origin/main
# verify variant count aligns:
rg -c "^\\s+[A-Z][A-Za-z]+," crates/argo-audit/src/event_kind.rs
cargo test -p argo-audit
cargo test -p argo-agent-runner --test truth_boundary_turn_exit
bash scripts/workspace-clippy-fast-fail.sh
git push --force-with-lease origin claude-mission-track-p-p-02-truth-boundary-turn-exit
```

## Owner

claude (Rust + runtime + audit surface change; not codex fast-mode mechanical)

## Reference

- Conductor rebase recipe on PR #2295: https://github.com/mark-hallam/Argo-Agent/pull/2295#issuecomment-4535308777
- Evaluator round-65 detailed analysis: `docs/skycot/evals/scheduled/2026-05-25/round-65-pr-2295-track-p-p-02-truth-boundary-blocked-conflict-and-loc.md`
- Dependency brief: `streams/fix-split-event-tests-rs-loc-budget-breach.md`
