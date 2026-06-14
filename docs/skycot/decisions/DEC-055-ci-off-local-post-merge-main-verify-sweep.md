# DEC-055 — CI stays off; a local post-merge sweep guards main integrity

**Status**: active
**Date**: 2026-06-14
**Operator-locked**: yes (this session)

## Decision

1. **GitHub Actions (CI — the robot that tests every change) stays permanently
   disabled.** The operator confirmed 2026-06-14 this is a permanent call at this
   stage ("I keep the github billing blocked. assume thats a permanent call").
   Design around it — do not re-enable GHA or assume it exists.
2. **main-branch integrity is guarded by a LOCAL post-merge verification sweep.**
   `scripts/main-verify-sweep.sh` (launchd `com.skycot.main-verify`) re-verifies
   the MERGED result of `origin/main` after every advance, in an isolated worktree
   (compile + curated critical verifiers). On RED it sets a merge-freeze sentinel
   (`~/.argo/main-verify/merge-freeze`) that `conductor-pre-merge-gate.sh`
   hard-refuses on — so the conductor cannot merge onto a known-broken main.

## Context

PRs are admin-merged to `origin/main` with **no robot re-checking the merged
result** (CI is off). On 2026-06-14 a reviewed + merged + test-green fix (#2936,
the approvals-convergence fix) was **silently reverted** by a later concurrent
merge whose merge-resolution took the pre-#2936 version of the file. `git log
-- <file>` hides this (history simplification hides a merge whose net effect vs
its first parent is "revert"), and nothing re-ran tests on main — so it went
unnoticed until a manual 25× browser e2e soak caught it.

That is the exact failure mode the product exists to prevent — truth-bounded,
verifiable, NO-THEATER ([DEC-029](DEC-029-truth-bounded-status-reporting.md)) —
reflected onto the build itself: **"merged" did not mean "still on main and still
working."** It is the [DEC-042](DEC-042-continuous-self-improvement-loop.md)
build-loop spine ("every loop ends in a CHECKABLE verifier — never a model's
self-report") applied to `origin/main`, which it previously was not.

## Why local, not GHA

The textbook fix for "main isn't verified" is CI-on-main. It is off here only
because of the billing block — a cost/access decision, now operator-locked
permanent. The worker swarm already runs `cargo` locally 24/7; the sweep points
that existing compute at main after each merge. Near-free, no GitHub dependency.

## What the sweep does

- Watches `origin/main`; on advance, syncs an **isolated** worktree
  (`conductor-build-worktree.sh`, dedicated target dir + shared `sccache` — disk
  cost, never cargo-lock contention with the swarm), then runs
  `main-verify-suite.sh`:
  - **Phase A**: `cargo check --workspace --tests` — the compile guard (the #1
    failure mode is a merge that doesn't build).
  - **Phase B**: curated critical verifiers (`main-verify-critical-tests.txt`) —
    the #2936 convergence **canary**, the mission-claim LIVE regressions
    (claims 1/2/4/5-non-CU/6), and the 17 `surface_contract` public-API pins.
    **A critical filter that matches ZERO tests is RED** — a clobbered or renamed
    verifier is the failure mode itself, so a vanished canary must not pass green.
- **GREEN**: records the verified SHA, clears the freeze.
- **RED**: sets the merge-freeze sentinel + `status.json` (both under `~/.argo`,
  immune to the workers' `git reset --hard` of the shared tree) + a best-effort
  `[main-verify-RED]` line in `SHARED.md`.
- `conductor-pre-merge-gate.sh` hard-refuses while frozen (the load-bearing
  guard, works even if the prompt drifts); `PROTOCOL_LOOP_PROMPT.md` §5 adds the
  conductor-prompt-level freeze-check.

## Consequences

- A silent revert of a pinned fix is caught within ~minutes (one main-advance),
  not days, and blocks further merges until green — closing the #2936 hole.
- The operator installs the launchd job once (committed template
  `deploy/launchd/com.skycot.main-verify.plist`; runbook
  `docs/skycot/operator/main-verify-sweep.md`). Pause via
  `touch ~/.argo/main-verify-stop`; it respects the swarm stop sentinels too.
- Cost: one extra isolated build/target tree on disk + a few minutes of compute
  per main-advance (shared `sccache` amortizes). Accepted by the operator ("do
  the work even if the cost looks heavy").
- Coverage is the **curated critical set** per-advance (fast + hermetic), not the
  full workspace test run; an optional nightly `--full` pass is a future
  extension. The curated list is the source of truth — add a verifier and it is
  guarded; a removed-but-expected verifier surfaces as a 0-match RED.

## Related

- [DEC-029](DEC-029-truth-bounded-status-reporting.md) — NO-THEATER / truth-bounded.
- [DEC-042](DEC-042-continuous-self-improvement-loop.md) — the loop spine this completes for the build side.
- [DEC-034](DEC-034-paying-ready-sprint-and-conductor-driven-run-mode.md) — operator-drives run mode (the conductor is the merge chokepoint the freeze gates).
