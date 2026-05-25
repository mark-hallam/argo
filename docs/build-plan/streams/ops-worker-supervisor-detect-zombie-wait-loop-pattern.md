# Stream ops-worker-supervisor-detect-zombie-wait-loop-pattern — detect + kill workers stuck in "Waiting for..." iteration loops

**Owner**: codex (bash/scripting; ~50-80 LOC).
**Depends on**: none.
**Authored against**: origin/main `be9206ef2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-worker-supervisor-detect-zombie-wait-loop-pattern`.

🟡 **P1 supervisor-hygiene.** Source: operator interactive session 2026-05-24T23:30Z. The worker holding `codex-cli-routes-approvals-json-split-helpers` ran 6h15m across 10+ iterations on mac-host, each iter ending with a result whose `.result` field was `"Waiting for clippy/tests to finish."` or similar — never opened a PR, burned ~$45, supervisor pulses stayed fresh so watchdog didn't kill it. The operator had to manually `kill 22771` to free the slot.

## Scope

Add a detection rule to `scripts/worker-supervisor.sh` (or a sibling watchdog script — find via `rg "worker-supervisor|watchdog" scripts/`) that:

1. **Reads the worker's ndjson log** at `~/.argo/worker-logs/<timestamp>-<brief-id>.ndjson` for the active claim.
2. **Counts consecutive `result` events** (lines with `"type":"result","subtype":"success"`) where the `.result` field text matches the pattern `^Waiting for|^Still waiting|^Monitor is armed|^Build cache is huge` (extensible regex).
3. **Threshold**: if 3+ consecutive such results AND total brief etime > 2h, kill the worker process tree (SIGTERM then SIGKILL after 5s) and emit a watchdog event to `/tmp/argo-watchdog-wait-loop-this-run`.
4. **Demote the ledger row** to TODO via the existing `claim-with-retry` release path with a salvage note (use the pattern from the 2026-05-24T23:35Z manual demote — see commit `b886bd9e2`).

The supervisor sees the worker as healthy because the worker's stdout pulses normally — what's actually happening is the agent loop is stuck in a `Wait for monitor → report-no-progress → loop` cycle. The fix is content-aware: peek the result text, not just the timestamp.

## Grep before write

```
rg -n "worker-supervisor|watchdog|pulse|hung-pids|stale-pids" scripts/
```

Locate the existing supervisor + watchdog code (referenced via `/tmp/argo-watchdog-hung-pids-this-run` artifact). Extend rather than parallel-author.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [supervisor / watchdog scripts]

## Files you may touch

- `scripts/worker-supervisor.sh` — add the wait-loop detector or hook into the existing watchdog.
- `scripts/worker-watchdog.sh` or similar (if exists) — add the detector here if cleaner.
- `scripts/worker-supervisor-wait-loop-tests.sh` (NEW) — bash smoke harness with 3 cases: (a) genuine waiting (passes 1 iter then ships) is NOT killed; (b) 3+ consecutive "Waiting for..." beyond 2h IS killed; (c) ndjson log missing → no-op (don't crash).

## Tests

- Smoke harness above. Use synthetic ndjson fixtures.
- Manual smoke: tail the existing `codex-cli-routes-approvals-json-split-helpers` worker log to confirm the detector would have caught PID 22771 within the first 2h after pattern emerged.

## Definition of done

- Detector lives in supervisor or watchdog, fires on the threshold above.
- 3 smoke harness cases green.
- README (or inline comment) documents the regex extensibility for future patterns.
- PR body includes a replay simulation against the 2026-05-24T17:13:20Z PID 22771 log (showing the detector would have killed at ~T19:13Z, not T23:35Z manual intervention).

Notes: don't kill workers that ship within 2h — the threshold is "long-running AND non-progressing". The combination matters. Workers can have legitimate waits.
