# Troubleshoot a stuck worker

Most worker stalls fall into one of three buckets:

1. The worker hasn't been scheduled.
2. An action is stuck in `pending` because no human responded.
3. The worker tripped its circuit breaker after consecutive
   executor failures.

## Run `argo doctor` first

```bash
argo doctor --json
```

`doctor` checks workspace policy, the SQLite store, channel
manifests, and provider credentials. If anything reports `WARN`
or `ERROR`, fix that before investigating the worker.

For the broader doctor surface (dashboards, watchers,
auto-remediation):

```bash
argo doctor watch --interval 30s
argo doctor fix --auto
```

`fix` only runs the safe-to-auto-remediate set. Unsafe
remediations require explicit `--issue` flags.

## Check the worker portrait

```bash
argo worker show <id> --json
```

The portrait carries identity, resolved policy lineage (workspace
→ roster → worker), spend, action stats, handoffs, skill
candidates, anomalies, and circuit-breaker state.

If the circuit breaker is `Open`, the worker auto-paused after
consecutive executor failures. Resume it with:

```bash
argo worker resume <id>
```

## Inspect pending actions

```bash
argo worker actions list --state pending --vertical <id> --json
```

Each pending action has a `requested_at` timestamp and a
`channel_dispatch` row. If `channel_dispatch` is empty, no notify
event was fired — most often because no channel is configured for
the action's risk class. Re-check
[approvals setup](./approvals.md).

## Replay a completed action

```bash
argo action replay <id>
```

Replay reconstructs the action's execution context offline and
re-runs the executor in dry-run mode. Useful for diagnosing a
flaky executor without burning real spend.

## Tail the audit feed

```bash
argo activity tail --tenant <id> --worker <id>
```

The activity feed subscribes to `AuditSink` and prints rows in
real time. If the worker is silent, the feed will be silent —
which often points back to "the worker hasn't been scheduled".

## Force one turn

If the worker is silent and `argo doctor` is clean, run a single
turn manually:

```bash
argo work serve --vertical <id> --once
```

If that completes without emitting an action, the worker's
charter or memory is the culprit. Inspect the persona / charter:

```bash
argo vertical describe <id>
```

## Recovery suggestions

After a session has failed actions, ask Argo for a post-mortem:

```bash
argo recover <session-id>
```

This walks the audit chain, calls the configured LLM, and returns
three ranked fix suggestions. Advisory only — no auto-apply.

## Last resort

The single biggest source of "stuck worker" reports is a
`<workspace>/.skycot/notify.toml` whose channel credentials
silently expired. Run:

```bash
argo doctor channels
```

…and re-check every channel that isn't `OK`.

## Next

[Operations → Operator runbook](../operations/runbook.md) covers
the canonical end-to-end troubleshooting playbook (from
`docs/skycot/RUNBOOK.md`).
