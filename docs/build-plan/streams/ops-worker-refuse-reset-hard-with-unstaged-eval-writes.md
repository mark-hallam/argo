# ops-worker-refuse-reset-hard-with-unstaged-eval-writes

**Owner**: unclaimed.
**Depends on**: nothing.
**Authored against**: origin/main `952e42591` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-worker-refuse-reset-hard-with-unstaged-eval-writes`.

🟢 **P2 — ops/safety guard.** Per evaluator round 38 (SHARED.md 2026-05-24T15:13Z), the write-side runbook landed via #2155 but the race vector remains alive: a worker session running `git reset --hard origin/main` while the evaluator has unstaged writes in `docs/skycot/evals/scheduled/<date>/` can silently destroy evaluator output that hasn't yet been committed. This brief lands the missing pre-flight guard.

## Why this brief exists

Round-37 evaluator handoff observed: the swarm process performs `git reset --hard origin/main` in two places — worker bootstrap (`scripts/worker-loop.sh`) and the supervisor's per-slot reset. Both run unconditionally. If an evaluator (separate process, same checkout, no isolation) is mid-write into `docs/skycot/evals/scheduled/<date>/<round>/findings.md`, the reset destroys those uncommitted writes.

Mitigation today: the new write-side runbook (#2155, `docs/skycot/agents/heartbeats/WRITE_SIDE_RUNBOOK.md`) tells evaluators to commit their output before sleeping. But the operator-pref-style fix is to add a guard on the destructive side, not just discipline on the write side.

## What this brief should do

Add a small pre-flight check in `scripts/worker-loop.sh` (and any other place that runs `git reset --hard`) that refuses to reset if there are unstaged or untracked writes under `docs/skycot/evals/` belonging to a path that matches `scheduled/*/round-*/` or contains a non-empty filename. If detected:

1. Print a loud STDERR warning naming the affected files.
2. Stash the writes under `git stash push -u -m "ops-guard:worker-loop:<timestamp>"` instead of destroying them.
3. Continue the reset.

OR (simpler): refuse to reset at all, exit non-zero, and let the operator inspect.

Pick whichever option matches the existing operator-pref pattern in `scripts/`. Stash-and-continue is closer to "do what I mean"; refuse-and-exit is closer to "fail loud."

## Files you may touch

- `scripts/worker-loop.sh` — add the pre-reset check.
- Any sibling script that does `git reset --hard`: `grep -rn "reset --hard" scripts/` to enumerate before editing.
- `scripts/tests/worker-loop-no-reset-with-eval-writes.bats` (new test file, if BATS is the existing test harness) — fixture that creates a fake unstaged eval write, runs the loop, asserts the file survives.

## Files you must NOT touch

- `docs/skycot/agents/heartbeats/WRITE_SIDE_RUNBOOK.md` — the write-side mitigation is already shipped. This brief is the destructive-side mitigation.
- Production Rust code — this is pure shell/ops.

## Acceptance criteria

- [ ] Worker loop refuses (or stashes) when unstaged eval writes are present.
- [ ] BATS or shellcheck-only test pins the new behavior.
- [ ] Existing worker-loop happy path still works (clean tree → reset proceeds).
- [ ] shellcheck clean on the modified scripts.
- [ ] BUILD_STATUS.md row updated.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## LOC budget

~50-120 LOC bash + test.
