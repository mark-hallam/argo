# ops-watchdog-reap-stuck-scheduled-task-pids — auto-reap orphaned scheduled-task runner PIDs

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-watchdog-reap-stuck-scheduled-task-pids`.

## Why this is P2

Watchdog rounds 70/71/72 each flagged 6 stuck `claude` scheduled-task runner PIDs (2373/14319/47534/58362/86635/93418) with etimes 2h-8h. These are scheduled-task **runner** processes (parent claude.app headless invocations), NOT `claude -p` workers, so the existing R5/R6 watchdog targets cannot kill them. They hold open API stream-json IPC and may be wedging session resources. The operator has had to `kill -KILL` them manually each tick.

## Scope

Add a "stuck scheduled-task runner" auto-reaper to `scripts/worker-supervisor-watchdog.sh`:
- Identify a stuck runner by: (a) command line matches `claude --print` or similar headless invocation pattern, (b) no `Brief:` in cmdline (so it's a runner not a worker), (c) etime > 30 minutes (configurable via `STUCK_RUNNER_ETIME_MIN`, default `30`).
- For each match, `kill -KILL` it.
- Log to SHARED.md with `[watchdog-reaper]` prefix + the killed PIDs.
- Rate-limit: don't kill more than 10 PIDs in one watchdog tick to avoid runaway.
- Operator can disable via `WATCHDOG_AUTO_REAP=0` (default: `1`).

## Files you may touch

- `scripts/worker-supervisor-watchdog.sh` — add the reaper branch.

## Files you must NOT touch

- The LaunchAgent plists for scheduled tasks themselves.
- `scripts/worker-supervisor.sh`.
- Anything under `crates/`.

## Acceptance criteria

- [ ] Identify a stuck runner via test fixture: spawn a `sleep 99999 &` process with a fake `claude --print` cmdline, set etime threshold to 1s, watchdog reaps it.
- [ ] Worker processes (with `Brief:` in cmdline) are NOT reaped regardless of etime.
- [ ] SHARED.md entry written on every reap with PIDs + cmdline snippets.
- [ ] `WATCHDOG_AUTO_REAP=0` → no reap.
- [ ] No more than 10 PIDs reaped per tick.

## Branch + PR

- Branch: `claude-ops-watchdog-reap-stuck-scheduled-task-pids` or `codex-…`.
- PR title: `ops(watchdog): auto-reap stuck scheduled-task runner PIDs (ops-watchdog-reap-stuck-scheduled-task-pids)`.

## Notes

- ~60-100 LOC bash.
- Test fixture is the load-bearing part — use a unique cmdline marker so the test process is unambiguously identifiable.
- Authorized under `--skip-clippy` per evaluator-pre-verified ops-* path.
