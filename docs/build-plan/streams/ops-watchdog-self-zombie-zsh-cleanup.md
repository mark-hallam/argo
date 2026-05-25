# ops-watchdog-self-zombie-zsh-cleanup

**Owner**: claude or codex.
**Authored against**: origin/main `838e1141a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-watchdog-self-zombie-zsh-cleanup`.

🟢 **P2 — operational hygiene; observed during conductor tick T08-05 (2026-05-25T09:08Z watchdog summary).**

The watchdog spawns subshell `zsh -c '...'` processes to do its hang-detection work (mapping claude PIDs to supervisor slots, reading lsof for log file paths). Some of these subshells get stuck and don't exit cleanly — observed: PIDs 50710 (etime 1:18:16) + 53431 still wedged hours after the tick that spawned them completed.

The watchdog flags them ("two prior watchdog zsh shells still wedged from earlier ticks — out of remediation allowlist; left alone") but does NOT clean them up because the remediation allowlist is conservatively scoped to known-spawn-pattern processes the watchdog WAS designed to kill.

## Scope

1. Identify the spawn site — likely `scripts/check-hung-pids.sh` or wherever the watchdog runs `zsh -c '<inline-script>'` for slot mapping (grep `pgrep -P "$sup_main"` + `lsof -p` patterns).
2. Add a **self-cleanup pass** to the watchdog routine that runs at the START of every tick: find any `zsh -c` processes whose parent died OR whose elapsed time exceeds the tick interval (~60s for typical hang-detection work), kill them with SIGTERM, then SIGKILL after 5s if still alive.
3. Detection recipe (zsh-specific): `pgrep -P 1 -f 'zsh -c.*pgrep.*claude.*--dangerously-skip-permissions'` (or similar pattern unique to the watchdog's inline script). Filter to elapsed > 60s.
4. Emit a `[watchdog-self-cleanup]` line to docs/skycot/agents/SHARED.md per killed process so the trail is auditable.
5. Add a hermetic test under `scripts/test/test-watchdog-self-zombie-cleanup.sh` that:
   - Spawns a zsh subshell matching the watchdog spawn-pattern + `sleep 600`.
   - Manually detaches its parent (or waits for parent exit).
   - Runs the cleanup pass with a 1s elapsed-threshold (instead of 60s) for test speed.
   - Asserts the zsh process is killed.
   - Asserts SHARED.md contains the `[watchdog-self-cleanup]` line.

## Acceptance criteria

- [ ] Watchdog routine has a self-cleanup pass that runs at every tick start.
- [ ] Spawn-pattern matching is precise (does NOT kill user's interactive shells, only watchdog-spawned subshells).
- [ ] Hermetic test in `scripts/test/test-watchdog-self-zombie-cleanup.sh` passes.
- [ ] PR body includes `ps aux | grep zsh` output before + after a manual test run to confirm cleanup.
- [ ] SHARED.md emission verified in the test.

## May-touch

- `scripts/check-hung-pids.sh` (or wherever the watchdog routine lives)
- `scripts/test/test-watchdog-self-zombie-cleanup.sh` (NEW)
- This brief file (status updates)

## Out of scope

- Extending remediation to non-watchdog processes (e.g. claude-worker-loop zombies — handled by R5/R6 rules)
- Changing the watchdog's tick cadence
- Touching the supervisor launchctl plist

## Effort

~50-100 LOC bash + test. ~30-60 min.

## Observation context

From 2026-05-25T09:08Z watchdog summary (operator-shared):
> "two prior watchdog zsh shells (PIDs 50710, 53431) still wedged from earlier ticks — out of remediation allowlist; left alone"

PID 50710 elapsed `01:18:16` at observation time, state `Ss` (sleeping session leader). Command was the watchdog's inline hang-detection eval script. These accumulate over time on long-running supervisor sessions.
