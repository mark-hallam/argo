# claude-worker-loop-zombie-detection-followup

**Owner**: claude.
**Depends on**: `claude-worker-loop-zombie-detection` MERGED (✅ via #2038).
**Authored against**: origin/main `0ccebd630` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-worker-loop-zombie-detection-followup`.

🟡 **P2 OPS-TOOLING GAP.** Source: tick-58 incident. PID 14468 (`web-workflow-test-run-gateway-persist`) zombied at 16m silent post-`result:success` — the textbook signature #2038 was designed to catch. **But #2038's defenses didn't fire** for this incident. The conductor had to manually kill + demote. Investigate why, then close the gap.

## Scope

Read the worker logs for the PID-14468 incident (logs in `~/.argo/worker-logs/` matching `*test-run-gateway-persist*.ndjson` around 2026-05-22T11:15Z-12:23Z). Determine which of the 4 zombie-detection conditions failed:

1. **Did the watchdog spawn?** (look for the background subprocess in the worker loop log).
2. **Did the log-staleness check fire?** (the 20-min threshold should have triggered at ~11:35Z; PID was killed at ~12:23Z, so ~48 min had elapsed).
3. **Did the SIGTERM-then-SIGKILL escalation actually attempt?** (look for `kill -TERM` / `kill -KILL` lines).
4. **Did the post-iteration ship-check run?** (look for `RELEASED by worker-loop zombie-detection` note attempt).

Likely root causes (in order of probability):

- **(a) Watchdog never spawned.** If the watchdog's `( while ... ) &` line had a syntax error or env-var issue, the background process exited silently. Check the `set -euo pipefail` interactions — bash's strict mode can kill backgrounded subshells unexpectedly.
- **(b) Watchdog spawned but didn't reach 4-condition match.** Maybe the pulse file was still touched (so condition #4 — pulse silent ≥120s — never matched). The bash sidecar `( while [ -f "$PULSE_FILE" ]; do touch ...; sleep 30; done ) &` keeps touching the pulse file from the bash side, INDEPENDENTLY of whether claude is alive. If bash is wedged but the pulse touch loop is still running, condition #4 never fires.
- **(c) Watchdog fired but SIGTERM didn't unwedge claude.** Even with the signal, claude's process state stayed in `SN` and stdout pipe didn't EOF. The bash pipeline stayed blocked.

For each likely root cause, the fix:

- **(a)** Fix the watchdog spawn line (add explicit `set +e` around it; verify with a synthetic zombie test).
- **(b)** Change condition #4 from "pulse file mtime stale" to "pulse file READER subprocess dead" — check whether the touch-loop PID is still running, not its file mtime. The touch loop is independent of bash's wedge state.
- **(c)** Add a SIGKILL fast-path: if SIGTERM hasn't moved the process state out of `SN` within 5s, escalate immediately to SIGKILL. Don't wait 30s.

Pick whichever root cause the logs prove, ship the fix, and add a regression test that pins it.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `scripts/claude-worker-loop.sh` — narrowly scoped: fix whichever zombie-detection gap the investigation surfaces. Don't refactor the watchdog architecture; just patch.
- `scripts/check-zombie-workers.sh` — narrowly scoped: if condition #4's heuristic changes, update the sidecar's matching logic.
- `scripts/check-zombie-workers-tests.sh` — extend smoke test with a synthetic zombie that matches the PID-14468 incident pattern.
- `docs/build-plan/streams/claude-worker-loop-zombie-detection.md` (the original brief) — narrowly scoped: add a "Known gap — see follow-up brief" note pointing here.

## Files you must NOT touch

- The claude invocation prompt block.
- The signal trap, quota guard, or post-iteration ship-check.
- Any production / runtime code outside scripts/.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] PR body includes the relevant log snippet from PID-14468's worker log proving which gate failed.
- [ ] PR body names the root cause (a / b / c / other) with evidence.
- [ ] Fix lands the targeted change only.
- [ ] Regression smoke test passes: synthetic zombie matching PID-14468's pattern is detected + killed + claim released within 30min of `result:success` event.
- [ ] No regression on existing `check-zombie-workers-tests.sh` smoke scenarios.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-worker-loop-zombie-detection-followup`
- PR title: `ops(worker-loop): close zombie-detection gap proven by PID-14468 incident (claude-worker-loop-zombie-detection-followup)`

## Notes for the implementer

- ~80-150 LOC. Mostly investigation + small targeted patch + one new test scenario.
- **Start with the logs.** Don't speculate — let the worker log tell you what fired and what didn't. `tail -200 ~/.argo/worker-logs/*test-run-gateway-persist*.ndjson | grep -E "zombie|watchdog|SIGTERM|SIGKILL|RELEASED"` is a good first query.
- **Root cause (b) is my prior** because of the bash sidecar architecture — the pulse file is touched by bash independently of claude's state, so "pulse silent" never happens unless bash itself dies. If that's it, the fix is changing condition #4.
- **Be careful with the post-iteration ship-check fix.** The ship-check (which runs AFTER claude exits) is what auto-released claims for the next worker. If condition #4 is broken, the watchdog never SIGTERMs claude, claude never exits, the pipe never closes, the ship-check never runs. Fixing condition #4 might be the single fix that closes the whole loop.
- Test with `bash scripts/check-zombie-workers.sh` after the fix to confirm the sidecar's report changes correctly for synthetic zombies.
