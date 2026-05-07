# Failure modes

The canonical failure-modes document is
[`docs/skycot/failure_modes.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/skycot/failure_modes.md).
This page is a navigation overview.

## What the platform protects against

| Risk | Mitigation |
|---|---|
| Unbounded spend | per-worker + tenant budget; auto-pause when over |
| Wrong action | auto-approval gate + signed channel decisions |
| Policy drift | resolved policy lineage stamped on every audit row |
| Memory poisoning | author-stamped writes + cross-tenant isolation |
| Skill regression | quality scoring + judge gate + A/B test before promote |
| Chain tampering | hash-linked audit rows + `audit verify` |
| Silent channel failure | `argo doctor channels` connectivity probes |
| Anomaly miss | Welford z-score detection on per-worker spend / actions |
| Stuck approval | quiet hours + handoff routing + escalation chain |

## What the platform does NOT promise

- **Bug-free executors.** A buggy MCP connector can still emit a
  bad action. The auto-approval gate gives you a chance to catch
  it; replay gives you a way to debug.
- **Provider correctness.** If the LLM hallucinates an instruction
  to call a tool, the platform faithfully passes it through to the
  approval gate.
- **Network confidentiality between channels and providers.**
  Channel callbacks use HMAC-signed tokens; providers use the
  scheme you configure (TLS / mTLS).
- **Cross-host orchestration.** Single-host today. Multi-host is
  on the roadmap.

## When a worker is wrong

```bash
argo worker pause <id>
argo worker actions cancel <id>
argo audit replay <session-id>
argo recover <session-id>
```

If the wrongness traces to a learned skill:

```bash
argo skill quality-recompute <name>
argo skill review reject <id>
argo feedback down <skill-id>
```

If the wrongness traces to memory:

```bash
argo memory grep <suspect-content>
argo memory <id> forget
```

`forget` redacts (not deletes) so the audit chain stays intact.

## When the audit chain breaks

Chain breaks are surfaced by `argo doctor` and by every
`argo audit query` invocation. If verify reports a break:

```bash
argo audit verify <session-id>
argo audit replay <session-id> --format raw | jq '.seq, .chain_break'
```

A real break is rare and load-bearing. Capture the audit export,
preserve the SQLite store, and file an issue with the dump.

## Anomaly detection

Two sources of anomaly signal:

```bash
argo worker anomalies        # action-rate Z-score
argo budget anomalies         # spend Z-score per (worker, hour-of-day)
```

Both use Welford rolling stats with z-score threshold. Flagged
rows are advisory — they don't auto-pause. Wire them through your
on-call channel for steady-state ops.

## When the gateway is sick

```bash
argo gateway status --json
argo gateway routes --json
argo doctor --json
```

If the gateway PID is alive but `/health` is failing, restart:

```bash
argo gateway stop
argo gateway start --bind 127.0.0.1 --port 8080
```

The gateway can also be exercised via the HTTP API directly — see
[Reference → CLI](../reference/cli.md) for the route catalog.

## See also

- [`docs/skycot/failure_modes.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/skycot/failure_modes.md)
  — canonical list of known failure modes + mitigations.
- [Operations → Operator runbook](./runbook.md) — the steady-state
  playbook.
