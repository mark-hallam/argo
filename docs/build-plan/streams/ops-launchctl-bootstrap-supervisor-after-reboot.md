# ops-launchctl-bootstrap-supervisor-after-reboot — document + script supervisor recovery post-reboot

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-launchctl-bootstrap-supervisor-after-reboot`.

## Why this is P3

CLAUDE.md "To relight the swarm" lists a 3-step recipe (rm halt sentinels, launchctl load, plist auto-loads). But there is no single `scripts/relight-swarm.sh` that bundles it. Operator has done this manually multiple times. A one-shot script + a documented runbook would reduce the time-to-relight from minutes to seconds and make handoffs cleaner.

## Scope

1. Write `scripts/relight-swarm.sh` that:
   - Removes `/tmp/argo-stop` + `~/.argo/stop` (idempotent).
   - Checks `launchctl print gui/$(id -u)/com.skycot.worker-supervisor` for current state.
   - If unloaded, `launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.skycot.worker-supervisor.plist`.
   - If loaded, `launchctl kickstart -k gui/$(id -u)/com.skycot.worker-supervisor` to recycle.
   - Confirms via `launchctl print` that the supervisor is alive + reports the PID.
   - Tails `~/Library/Logs/argo-supervisor.log` (or wherever the supervisor logs) for 5s to confirm a worker iteration starts.
2. Add a runbook section to `docs/skycot/operator/relight-runbook.md` (new file).
3. Reference the script from CLAUDE.md "To relight the swarm" section.

## Files you may touch

- `scripts/relight-swarm.sh` — new.
- `docs/skycot/operator/relight-runbook.md` — new.
- `CLAUDE.md` — add a "→ `scripts/relight-swarm.sh`" pointer near the existing 3-step recipe.

## Files you must NOT touch

- The LaunchAgent plist itself (operator-owned).
- `scripts/worker-supervisor.sh`.
- Anything under `crates/`.

## Acceptance criteria

- [ ] `bash scripts/relight-swarm.sh` is idempotent (running twice produces the same end state).
- [ ] Script reports the supervisor PID + the first worker iteration tail.
- [ ] Runbook covers: when to run, what to verify after, when to escalate.
- [ ] CLAUDE.md pointer added.

## Branch + PR

- Branch: `claude-ops-launchctl-bootstrap-supervisor-after-reboot` or `codex-…`.
- PR title: `ops(supervisor): one-shot relight-swarm script + runbook (ops-launchctl-bootstrap-supervisor-after-reboot)`.

## Notes

- ~60-100 LOC bash + ~300 word runbook.
- Authorized under `--skip-clippy` per evaluator-pre-verified ops-* path.
