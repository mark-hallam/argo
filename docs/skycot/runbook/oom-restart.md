# OOM-kill detection + worker restart

Operator runbook for the `argo-runtime::oom_restart` supervisor
primitive (mission-oom-restart-handler, MISSION_BACKLOG §8
Claim 8 #237 — Tier P reliability).

## What it does

When a worker subprocess dies, the supervisor calls
`oom_restart::classify_exit` to derive a typed
`WorkerExitReason`, asks an `OomRestartCoordinator` what to do
next, and appends a `worker.oom_restart` audit-chain row.
Outcomes:

- **`Restart { after_backoff, attempt }`** — sleep, respawn.
- **`GiveUp { salvage_note }`** — budget exhausted within the
  rolling window; the supervisor releases the worker's brief
  claim back to `TODO` with `salvage_note` written into the
  ledger row.
- **`IgnoreCleanExit`** — `exit code 0` (or non-137 numeric
  exits); the existing clean-shutdown path runs.

The primitive is the **whole-process** path. Per-action
transient retries live in
`crates/argo-executor-computer-use/src/restart.rs` and are
distinct — touching one does not affect the other.

## Read recent restart history

```bash
argo worker restarts --since 1h
argo worker restarts --worker wkr-7b9 --since 30d
```

Output is JSON-lines, one row per audit entry, sorted newest
first. Each row carries:

```json
{
  "created_at_micros": 1715512345678901,
  "payload": {
    "worker_id": "wkr-7b9",
    "exit_reason": {"kind": "oom_killed", "signal": 9, "cgroup_evidence": "oom_kill 3"},
    "decision": {"kind": "restart", "after_backoff": {"secs": 4, "nanos": 0}, "attempt": 2},
    "attempt_count": 2,
    "recorded_at": "2026-05-12T14:32:25.678Z"
  }
}
```

Pipe through `jq` for richer filtering, or `argo audit show
--kinds worker.oom_restart` for the same data through the
audit-chain viewer.

## Tune the restart budget

Defaults: **3 restarts per 15 min** rolling window, `2s` base
backoff doubling to a `2min` ceiling.

```rust
let coord = OomRestartCoordinator::new(OomRestartConfig {
    max_restarts: 5,
    base_backoff: Duration::from_secs(5),
    max_backoff: Duration::from_secs(60),
    restart_window: Duration::from_secs(30 * 60),
});
```

Operator guidance:

| If you see…                                | Knob to turn                       |
| ------------------------------------------ | ---------------------------------- |
| The same worker thrashing restart-restart  | Lower `max_restarts` (force GiveUp |
|                                            | sooner so a human looks)           |
| Brief loss every time a worker briefly     | Raise `restart_window` (so the     |
| spikes memory                              | budget covers a longer steady-     |
|                                            | state)                             |
| Cold caches making the first restart fail  | Raise `base_backoff` (let the host |
|                                            | settle before respawning)          |
| Backoff growing past your SLO              | Lower `max_backoff` (cap retry     |
|                                            | latency)                           |

The exponential backoff is `base * 2^(attempt-1)`, clamped at
`max_backoff`. Attempt counter is 1-based — the first restart
inside the window pays `base_backoff`.

## When to escalate to per-worker memory caps

If `argo worker restarts --worker <id> --since 7d` shows the
same worker repeatedly OOM-killed, restarting in isolation is
papering over a sizing problem. Open a follow-up against
**#236 (`Per-worker resource limits`)** to add a cgroup memory
cap rather than continuing to ride the restart loop. Indicators:

- Same `worker_id` with 5+ OOM rows in 7 days.
- Restart attempts consistently exhaust the budget (frequent
  `decision.kind = "give_up"`).
- `payload.exit_reason.cgroup_evidence` shows an `oom_kill`
  counter incrementing each row — confirms the kernel reaper,
  not a logic-level abort.

## When the give-up path fires

`RestartDecision::GiveUp` releases the brief claim. Operator
action depends on what the salvage note says:

1. `last exit=OOM (signal 9)` — the worker is starving. Bump
   memory budget (see #236) before re-queuing.
2. `last exit=signal 11` (SIGSEGV) — the worker code crashed.
   Pull the worker's stderr tail from sealed-debug; do NOT
   re-queue the same brief without a fix.
3. `last exit=signal 15` (SIGTERM) — supervisor delivered a
   clean shutdown but the worker did not exit gracefully.
   Check supervisor logs; this is usually a deadlock or a
   blocking syscall.
4. `last exit=hung (last heartbeat ...)` — the process was
   alive but stuck. Inspect `/proc/<pid>/wchan` or attach a
   debugger; this rarely fixes itself.

In every case, the ledger row's salvage note quotes the most
recent 3 attempts so the operator can trace the pattern
without re-querying audit history.

## Platform matrix

| Platform | OOM signal     | Code path                         |
| -------- | -------------- | --------------------------------- |
| Linux    | signal 9       | `ExitStatus::signal() == Some(9)` |
| Linux    | shell exit 137 | `ExitStatus::code() == Some(137)` |
| macOS    | signal 9       | `ExitStatus::signal() == Some(9)` |
| WSL2     | signal 9       | Linux path                        |
| Windows  | n/a            | Falls through to `OtherSignal`    |

On Linux, the classifier best-effort reads
`/sys/fs/cgroup/<cgroup>/memory.events` and extracts the
`oom_kill <count>` line into `cgroup_evidence`. The read is
optional — workers not in a cgroup-v2 hierarchy still classify
correctly, just without the evidence string.

## Sealed-debug + credential hygiene

The audit row deliberately does **not** capture the worker's
stderr. Worker logs can carry credential fragments
(provider tokens, MCP bearer headers); piping them into the
hash-chained audit row would persist secrets indefinitely.

For deep triage, capture the worker's stderr to sealed-debug:

```bash
argo session show <session_id> --include-sealed-debug \
    | jq '.stderr_tail'
```

Sealed-debug rotates aggressively (default 24 h) and is
operator-gated, which is the right blast radius for credential-
adjacent strings.

## No notification channel in v1

`worker.oom_restart` writes to the audit chain but does not
fan out to Slack / email / PagerDuty. If you want active
alerting, build a small consumer against the audit-stream
broadcast (`/audit/stream` gateway endpoint with
`kind=worker.oom_restart`). A follow-up brief will package this
once an operator asks — adding it speculatively would couple
the supervisor to a channel substrate the swarm has not yet
chosen.

## See also

- Brief: `docs/build-plan/streams/mission-oom-restart-handler.md`
- Per-action retries: `crates/argo-executor-computer-use/src/restart.rs`
- Per-worker memory caps (follow-up): MISSION_BACKLOG §8 #236
- Sandbox-side reaper: `mission-e2b-reap-idle` (orphan e2b sandboxes)
