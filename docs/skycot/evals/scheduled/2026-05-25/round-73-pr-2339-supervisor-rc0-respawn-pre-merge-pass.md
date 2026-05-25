# Round 73 — PR #2339 supervisor rc=0 respawn: PRE-MERGE PASS (ω-r68 RECURRENCE-6 CLOSER)

- **Date**: 2026-05-25 (Melbourne 2026-05-26 08:30 AEST)
- **Round**: 73 (scheduled-task hourly cron — addendum scenario after conductor T22-07 explicitly held #2339 awaiting evaluator PRE-MERGE PASS)
- **Pillar / Claim**: process observability (ω-r68 6-round closer; supervisor lifecycle)
- **Verdict**: 🟢 PRE-MERGE PASS · P1 (ready to merge)
- **Headline**: PR #2339 (`fix(supervisor): respawn slot after clean rc=0 exit`) at head `15388f861` is MERGEABLE/CLEAN. The PR adds a new `wait_for_stop_clear` function in `scripts/worker-supervisor.sh` that BLOCKS slot subshells on STOP-sentinel presence (instead of the old `exit 0` that killed the slot permanently), plus a new pre-merge gate (Gate 7/8) that runs the included smoke test `scripts/test-worker-supervisor-rc0-respawn.sh`. Ran the smoke test against the PR head — **PASS in 4.137s wall** (`worker-supervisor rc=0 respawn test passed` rc=0). The fix is the literal root-cause closure for ω-r68 RECURRENCE-1 through RECURRENCE-6 (slot-1 dead-since-15:02:33Z because the old supervisor honored STOP-sentinel by exiting, then never respawned).

## Setup

```bash
git fetch origin pull/2339/head:pr-2339-test --force --quiet
git checkout pr-2339-test --quiet
git rev-parse HEAD                                  # 15388f861772cb7a6fac75b4694e226e86b0e9d8
time bash scripts/test-worker-supervisor-rc0-respawn.sh
```

PR scope (per `gh pr view 2339 --json`):
- 3 files: `scripts/conductor-pre-merge-gate.sh`, `scripts/test-worker-supervisor-rc0-respawn.sh` (NEW, 89 LOC), `scripts/worker-supervisor.sh`
- +145 / -26
- State: CLEAN / MERGEABLE
- Title: `fix(supervisor): respawn slot after clean rc=0 exit — closes ω-r68 RECURRENCE-4 (fix-worker-supervisor-respawn-after-rc-0)`

## Observed

```
=== HEAD ===
15388f861772cb7a6fac75b4694e226e86b0e9d8

=== Run smoke test ===
worker-supervisor rc=0 respawn test passed
bash scripts/test-worker-supervisor-rc0-respawn.sh  0.07s user 0.10s system 4% cpu 4.137 total
rc=0
```

The smoke test:
1. Creates a counter file + start-log + fake-worker that increments the counter, creates STOP sentinel on first iter, sleeps 1s, exits rc=0.
2. Spawns `scripts/worker-supervisor.sh 1` with env-var overrides pointing at the fake worker + temp sentinel paths + 1s polling intervals.
3. **Phase A** — waits for `count ≥ 1 AND STOP sentinel present` (proves worker ran iter 1 and created the sentinel), then removes the sentinel.
4. **Phase B** — waits up to 60s for `count ≥ 2` (THE load-bearing assertion: did the supervisor RESPAWN the slot after rc=0 + STOP-sentinel clear?).

Test completed in 4.137s — phases A + B both passed.

## Anti-theater check

Three independent reasons this isn't "test rewrites itself green":

1. **The test would FAIL on the old supervisor.** Old `worker-supervisor.sh` had `if stop_present; then echo "halting"; exit 0; fi` at every check point (lines 311, 338, 423 of the pre-PR file). Under that behavior: Phase A removes the sentinel AFTER the slot subshell has already exited rc=0; supervisor never respawns the slot subshell (the supervisor doesn't have a "restart slot loop" mechanism — the `for slot in ...` loop forks each subshell ONCE at startup); count stays at 1; Phase B times out at 60s with `worker-supervisor rc=0 respawn test FAILED`. The new behavior `wait_for_stop_clear $slot $phase; continue` keeps the subshell ALIVE, blocking on the sentinel, then re-entering the spawn loop when cleared.
2. **The smoke test is wired into the pre-merge gate as Gate 7/8.** The same PR adds `bash scripts/test-worker-supervisor-rc0-respawn.sh` to `scripts/conductor-pre-merge-gate.sh:163-170`, with a `--skip-supervisor-test` escape hatch ONLY for non-supervisor-touching PRs. So this test is now part of the standing merge gate, not a one-shot verification.
3. **All env-var overrides are NEW in this PR.** Old supervisor hard-coded `WORKER_SCRIPT="$REPO/scripts/claude-worker-loop.sh"`, `STOP_SENTINEL_TMP="/tmp/argo-stop"`, etc. — there was NO way to drive it with fakes. The new ENV-var indirection lines 41-47 (`WORKER_SCRIPT="${WORKER_SCRIPT:-$DEFAULT_WORKER_SCRIPT}"` etc.) are precisely what the smoke test depends on. So the test couldn't exist without the production-code change — they ship together.

Additional sanity: the test runs the REAL `worker-supervisor.sh` (not a separate impl), so the smoke test exercises the actual `wait_for_stop_clear` function being added.

## Gap / follow-up

**None blocking merge.** The conductor T22-07 entry in SHARED.md explicitly says: *"Held back: #2339 (...). Behavior change in supervisor → defer to evaluator round-73 PRE-MERGE PASS before merging."* — this report supplies that PASS. Next conductor tick should merge under overnight `--skip-clippy` ops-* authorization (per CLAUDE.md memory `feedback_skip_clippy_authorized_for_evaluator_pre_verified_prs`).

**Post-merge observation for evaluator round 74**: re-check the watchdog SHARED.md alert. The expected sequence after merge:
1. PR #2339 lands → new `worker-supervisor.sh` deployed to local checkout
2. Operator (or post-merge hook) does `launchctl kickstart -k gui/$(id -u)/com.skycot.worker-supervisor` to recycle the running supervisor
3. New supervisor spawns slot-1 + slot-2 — slot-1 now alive
4. Watchdog at T+1h sees `slots=2/2` → degraded-run streak resets from 6 to 0
5. ω-r68 closes after RECURRENCE-6 — total escalation: 6 watchdog rounds + 6 evaluator rounds (r68→r73)

If step 2 doesn't happen automatically, that's a follow-up brief candidate: `ops-supervisor-kickstart-on-supervisor-script-merge` (parallel to #2336 `auto-restart-gateway-on-schema-bump`). Defer to evaluator round 74's process-drift observations.
