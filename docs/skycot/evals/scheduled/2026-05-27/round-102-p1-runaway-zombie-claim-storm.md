# Round 102 — P1 runaway zombie-claim storm

**Date**: 2026-05-27 (UTC tick 13:29).
**Round**: 102.
**Pillar/Claim**: Process drift — swarm protocol hygiene (orthogonal to mission claims).
**Verdict**: 🔴 FAIL — **P1 severity** — active storm at time of run.

## Setup

- `git fetch origin main && git log --pretty="%s" origin/main -200 | grep -c "zombie; no PR opened"`.
- `git log --pretty="%s" origin/main -200 | grep "zombie; no PR opened" | sed 's/.*release //;s/ (zombie.*//' | sort | uniq -c | sort -rn`.
- `grep -n "fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira" docs/build-plan/BUILD_STATUS.md` — read the ledger row to inspect claim history.
- `gh pr list --state all --search "fix-vertical-landing-non-canonical-worker-names" --json number,state,title --limit 10` — confirm no PR was ever opened.
- `grep -nE "(per.brief|brief.history|claim.count|brief.attempt|throttle)" scripts/worker-supervisor.sh` — confirm zero per-brief throttle exists.
- `pgrep -fl "claude-worker-loop|worker-supervisor"` — confirm 3 supervisor processes are healthy and actively spawning.

## Observed

- **99 zombie release commits in last 200 commits on origin/main** (`status: release <brief-id> (zombie; no PR opened)`).
- Per-brief breakdown:

  ```text
    83 fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira
    12 fix-audit-list-kind-query-param-ignored
     4 fix-vertical-landing-cta-and-step-copy-worker-to-teammate
  ```

- The most-zombied brief alone accounts for 83 of 99 cycles — concentrated on a single TODO row.
- `BUILD_STATUS.md:371` ledger row for `fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira` contains **49 consecutive `claim-with-retry` entries between 2026-05-27T13:02:08Z and 2026-05-27T13:18:59Z** — one claim every 20-30 seconds for 16 minutes:

  ```text
  | fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira | claude | TODO | — |
    claimed via claim-with-retry on mac-host at 2026-05-27T13:18:59Z.
    claimed via claim-with-retry on mac-host at 2026-05-27T13:18:29Z.
    claimed via claim-with-retry on mac-host at 2026-05-27T13:18:12Z.
    ... (49 entries total) ...
    claimed via claim-with-retry on mac-host at 2026-05-27T13:02:08Z.
  ```

- `gh pr list --state all --search "fix-vertical-landing-non-canonical-worker-names"` returned `[]` — no PR was ever opened, not even closed-without-merge. The brief simply hasn't been shipped, but the worker pool has been continuously consuming it for hours.
- `grep -nE "(per.brief|brief.history|claim.count|brief.attempt|throttle)" scripts/worker-supervisor.sh` returned **no matches** — `scripts/worker-supervisor.sh` (464 LOC) has zero per-brief throttle / claim-rate-limit / brief-history logic.
- Supervisor processes alive and spawning normally: `PID 85450 / 85524 / 85525 bash scripts/worker-supervisor.sh 2`.
- Brief itself (`docs/build-plan/streams/fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira.md`) is a **legitimate small web-only ask** — swap 3 worker names in `web/app/for/_data/verticals.ts` (~10 LOC), authored against SHA `9c407cd20039adf25a986343b7c8cdba76650a52`, freshness header present, acceptance criteria clear. The brief is NOT malformed — the failure is downstream.

## Root cause

Workers pick the **first eligible `TODO` row by document order** in `BUILD_STATUS.md` (per `docs/build-plan/CONTRIBUTING_AGENT.md` step 2). When a worker iteration fails to ship — for any reason — `scripts/claude-worker-loop.sh:696-730` (post-iteration ship-check) correctly demotes the row back to `TODO` with a release commit `status: release <brief-id> (zombie; no PR opened)`.

Nothing prevents the next supervisor iteration from immediately re-claiming the same `TODO` row that just zombie-released. Workers pick first-eligible-TODO; the brief is at line 371 (near top of queue); it just got re-marked TODO 20 seconds ago; so the new iteration claims it; same failure; cycle repeats indefinitely.

The `scripts/claude-worker-loop.sh` zombie-detection watchdog at lines 445-547 correctly handles **intra-iteration** zombies (claude wedged in `SN` state mid-iteration). It does **not** address **cross-iteration** runaways where each iteration exits cleanly but the next one immediately re-tries the same broken brief.

## Anti-theater check

This is a 🔴 FAIL finding, not a 🟢 PASS — anti-theater concerns invert (proving a defect is real, not proving a fix isn't fake). Specifically:

1. The 99 zombie commits are observable directly in `git log` — not just in ledger annotations that could be spoofed.
2. The 49 consecutive claim-with-retry entries are in the live ledger on origin/main (`BUILD_STATUS.md:371`); the supervisor PIDs are alive and actively spawning new iterations as I write this.
3. `gh pr list --state all` shows zero PRs across all states for the brief — the worker pool has consumed at least 99 iterations (likely more, since the 200-commit window only goes back ~hours) and produced zero PRs. Resources spent, no output.
4. R91 (yesterday) observed the same pattern at ~25 cycles on PR #2429 before that brief successfully shipped. Today's storm is 3× larger on a single brief and has been running continuously for hours with no terminal state — this isn't a one-off.
5. The `grep -nE "(per.brief|brief.history|claim.count|brief.attempt|throttle)" scripts/worker-supervisor.sh` returning empty is direct file-system proof that no preventive logic exists.

## Gap / follow-up brief candidates

### ✅ AUTHORED THIS ROUND — P1

**`ops-supervisor-block-runaway-zombie-claims-on-brief`** — promoted from R91's P2 to P1 given the active storm is 3× worse than the original trigger.

- **File**: `docs/build-plan/streams/ops-supervisor-block-runaway-zombie-claims-on-brief.md` (authored inline; freshness check PASS against `cb264f32f`).
- **Owner**: claude or codex (lightweight bash; either pool can ship).
- **Estimated size**: ~80-120 LOC bash + ~40-60 LOC hermetic smoke test.
- **Scope**: 3 new bash scripts (`scripts/check-runaway-zombie-claims.sh`, `scripts/check-runaway-zombie-claims-tests.sh`, `scripts/demote-to-blocked.sh`) + ~20-30 LOC integration into `scripts/worker-supervisor.sh` (current 464 LOC; LOC budget stays ≤500). Two-tier guard: 3 zombies in 30min → skip-claim this iter (let row drop); 6 zombies in 30min → force-demote row to `BLOCKED` with salvage note and require manual conductor/operator review.
- **First acceptance criterion (regression test that turns 🟢)**: the new hermetic smoke harness exits 0 with all 6 fabricated cases (clean / below-threshold / throttle / block / outside-window / multi-brief) passing.
- **Parallel-surface check**: PR #2278 `ops-worker-supervisor-detect-zombie-wait-loop-pattern` is the **intra-iteration** wait-loop detector (different defect — kills workers stuck mid-iteration emitting `^Waiting for clippy/tests to finish`). This new brief is the **cross-iteration** runaway detector. They coexist cleanly.
- **Out of scope**: stopping the current live storm (operator/conductor manual remediation), per-brief root-cause investigation, dashboards/Slack/observability.

### ⚠️ OPERATOR-SURFACE — manual remediation needed THIS TICK

The currently-running storm will not stop until either the supervisor is taught to throttle (the brief above ships) or the two storm rows are manually demoted. Recommend the conductor's next interactive tick (or operator) manually flip the two storm rows to BLOCKED:

```text
fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira  (BUILD_STATUS.md:371)
fix-audit-list-kind-query-param-ignored                          (BUILD_STATUS.md:373)
```

Salvage note text: `**BLOCKED by operator/conductor manual remediation 2026-05-27T13:30Z:** runaway zombie-claim pattern (83+ cycles in 200 commits); needs per-brief root-cause investigation before re-claim; see docs/build-plan/streams/ops-supervisor-block-runaway-zombie-claims-on-brief.md.`

Flagging this in the SHARED.md handoff so the next conductor tick picks it up.

### Not authored — out of scope for evaluator pilot

- **Per-brief root-cause** of why `fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira` fails to ship — needs separate investigation by whoever claims the BLOCKED row. Possible causes: brief acceptance criteria failing (vitest regression), worker quota issues, network partition during `gh pr create`. Cannot be diagnosed from ledger-level evidence alone.

## Verdict

🔴 FAIL — P1 — process-drift defect actively burning worker quota on a single TODO row at one claim every 20-30 seconds for hours, with zero ship output. R91's P2 recommendation has now bitten 3× worse on a different brief; P1 brief authored inline this round per pilot.
