# fix-worker-supervisor-respawn-after-rc-0 — supervisor stops respawning slot-1 after a clean exit (ω-r68 RECURRENCE-4)

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `d5b7cc458` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-supervisor-respawn-after-rc-0`.

## Why this is P2 (ω-r68 — recurrence-4, escalating)

Evaluator round-71 escalated ω-r68 to RECURRENCE-4: slot-1 of the LaunchAgent-managed `com.skycot.worker-supervisor` has been MISSING for 5+ hours despite 4 consecutive degraded watchdog runs (the loudest possible operator-intervention signal short of paging). The brief to fix the supervisor's respawn-after-rc-0 path has been UNAUTHORED for 4 rounds (r68→r71), so this brief authors it.

The supervisor's rc=0 branch (`scripts/worker-supervisor.sh:381-385`) currently treats rc=0 as "clean exit, respawn after 30s backoff". The observed failure mode: after a worker exits cleanly (spin-loop guard or signal trap), the slot subshell either (a) hangs on `wait` indefinitely, (b) silently exits the outer `while true; do … done` block, or (c) the `set -e` near line 367 trips on some downstream conditional and kills the slot subshell without respawn. The watchdog can see "slot-1 absent" but cannot recover it because the subshell `(…) &` is gone, not just idle.

Round-71 source: `docs/skycot/evals/scheduled/2026-05-25/round-71-process-drift-sweep.md` (ω-r68 RECURRENCE-4).

## Scope

One-part diagnosis + fix to `scripts/worker-supervisor.sh`.

1. **Reproduce locally** (~5 min): start the supervisor with `N_SLOTS=1` (so the failure mode is isolated), force a clean rc=0 exit from the worker (e.g. `touch /tmp/argo-stop` then `rm /tmp/argo-stop` after 30s — the worker should respawn). Observe whether the slot subshell exits the outer loop. If it doesn't reproduce, instrument the loop with `set -x` and a per-iteration log line and re-run for ~10 minutes to catch the natural failure.

2. **Diagnose** the failure mode against the three hypotheses (`wait` hang, silent loop exit, `set -e` trip).

3. **Fix** with the minimal change. Likely candidates:
   - Wrap the entire slot subshell body in a `while true; do … done` outer guard with `set +e` around the worker invocation + RC capture (already partially done at lines 358-367, but the `set -e` after `set +e` at line 367 might trip on a non-zero conditional later).
   - Replace `wait "$WORKER_PID"` with a polled loop (`while kill -0 "$WORKER_PID" 2>/dev/null; do sleep 1; done`) to defang the `wait` hang hypothesis.
   - Add a per-slot heartbeat write so the watchdog can distinguish "slot subshell alive but idle" from "slot subshell dead".

4. **Add a self-test** at `scripts/test-worker-supervisor-rc0-respawn.sh` (~30 LOC bash) that:
   - Forks a fake worker that exits rc=0 after 5s.
   - Runs the supervisor against it for 60s.
   - Confirms ≥2 respawns happened (i.e. the fake worker started, exited, and the supervisor started it again).
   - Exits non-zero if respawn didn't happen.

5. **Wire the self-test into `scripts/conductor-pre-merge-gate.sh`** as a new step (after the existing tests, before clippy) — guarded by a `--skip-supervisor-test` flag so it doesn't slow every gate run, but available when the supervisor changes.

**Not in scope**: changing the LaunchAgent plist, changing N_SLOTS default, modifying the watchdog itself, changing the worker script, broader process-management refactor.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `scripts/worker-supervisor.sh` — the fix.
- `scripts/test-worker-supervisor-rc0-respawn.sh` — new file, the self-test.
- `scripts/conductor-pre-merge-gate.sh` — wire the self-test in (additive).

## Files you must NOT touch

- `~/Library/LaunchAgents/com.skycot.worker-supervisor.plist` — operator-owned.
- `scripts/claude-worker-loop.sh` — out of scope (worker behaviour is correct; supervisor is the bug).
- `scripts/worker-supervisor-watchdog.sh` — out of scope.
- Any file under `crates/`.

## Acceptance criteria

- [ ] `bash scripts/test-worker-supervisor-rc0-respawn.sh` exits `0` on a clean repo (supervisor respawns the fake worker ≥2 times in 60s).
- [ ] A documented root-cause-and-fix block in the PR body (which hypothesis was correct + the one-paragraph rationale for the fix).
- [ ] `bash scripts/conductor-pre-merge-gate.sh --skip-clippy` passes (additive scope only).
- [ ] BUILD_STATUS.md updated with PR number on flip to IN_REVIEW.

## Branch + PR

- Branch: `claude-fix-worker-supervisor-respawn-after-rc-0` or `codex-fix-worker-supervisor-respawn-after-rc-0`.
- PR title: `fix(supervisor): respawn slot after clean rc=0 exit — closes ω-r68 RECURRENCE-4 (fix-worker-supervisor-respawn-after-rc-0)`.

## Notes for the implementer

- ~30-60 LOC bash. Fast-mode candidate.
- The current supervisor body (lines 297-417) is dense — read top-to-bottom before patching.
- Bash gotcha: `set -e` inside a `(...) &` subshell kills the subshell on any non-zero, including `[ "$x" -eq 0 ]` if `$x` is unset. The slot subshell uses `set +e` around the worker invocation but resumes `set -e` immediately — any later conditional that returns non-zero (e.g. the RAM-pressure check at line 321) could be tripping the subshell.
- Reference: `docs/skycot/evals/scheduled/2026-05-25/round-71-process-drift-sweep.md` for the ω-r68 finding history (rounds 68/69/70/71).
- After this brief ships, the next watchdog run should report `streak=0 healthy` instead of `streak=4 degraded slot-1 MISSING`. That's the load-bearing real-world signal.
