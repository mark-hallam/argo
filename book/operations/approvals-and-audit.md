# Approvals and audit

Two interlocking systems operators interact with daily.

## Approvals

Every action runs through the auto-approval gate before the
executor touches anything. The gate composes:

- Auto-approval policy (workspace ← roster ← worker)
- Spend budget (per-worker daily / monthly + tenant quota)
- Signature requirements (1+ channel sigs for high-risk actions)
- Quiet hours (always lowest-level applies)

When the gate returns `Decision::Ask`, Argo dispatches a notify
event to every channel in `notify.toml`. The action stays
`pending` until any channel's decision callback flips it.

### Channels

| Channel | Decision route |
|---|---|
| `push` | Deep-link → `/decisions/<token>` |
| `email` | Inbound webhook (Postmark) |
| `slack` | Block Kit buttons → HMAC-signed callback |
| `sms` | Reply token (HMAC) |
| `voice` | Twilio DTMF (1=approve, 2=deny, 9=details) |
| `imessage` | Blue Bubbles bridge (1/2/9) |
| `whatsapp` | Twilio reply (approve/deny keyword) |
| `webinbox` | Embedded HTML inbox + WS push |

Set them up per the [How-to](../howto/approvals.md).

### Multi-channel

A worker can require **N signatures** before an action moves
`pending → approved`. Set on the worker's auto-approval policy:

```toml
[auto_approval]
require_sigs = 2
```

Channels accumulate signatures; the action moves once the
threshold is hit.

### Quiet hours

Always at the lowest level (worker > roster > workspace). During
quiet hours, the gate auto-escalates regardless of confidence —
no autonomous action is taken until the next allowed window.

## Audit

Every transition writes an audit row. Rows are hash-linked; chain
integrity is verifiable.

### Querying

```bash
argo audit query --session <id> --json
argo audit query --decision approved --since 1d
argo audit query --worker <id> --escalation
argo audit query --kind handoff_settled
```

Filters compose. Output is JSON-line by default; `--format
markdown` for a readable summary.

### Replay

```bash
argo audit replay <session-id>
argo audit replay <session-id> --format raw
argo audit replay <session-id> --from-seq 5 --until 30
```

Replay walks the chain in `seq` order. Pretty format is the
default; `--format raw` emits the underlying NDJSON.

### Reports

```bash
argo audit report --since 7d --format markdown --output weekly.md
argo audit report --format html --output dashboard.html
```

Six sections: summary stats / channel breakdown / top spenders /
pending queue / chain integrity / known flakes.

### Real-time stream

```bash
argo activity tail --tenant <id>
```

Subscribes to `AuditSink` and streams new rows as they're
written. Filters: `--worker`, `--since`, `--include-skill`,
`--limit`.

## Compliance

GDPR delete redacts (not deletes) across memory / audit / skills /
channels / workers / sessions. The audit chain stays intact —
redacted rows still hash-link.

```bash
argo gdpr delete --subject user@example.com --dry-run
argo gdpr delete --subject user@example.com --confirm \
  --bundle-out gdpr-2026-05-07.tar.gz
```

Data classification (PII / PHI / Financial / Secret / Public /
Untagged) tagged at rest:

```bash
argo classify set <row-id> --class Pii
argo classify list --expired
argo classify rules
```

Per-class retention defaults: PII 90d, PHI 365d, Financial
730d, Secret 30d.

## See also

- [`docs/skycot/RUNBOOK.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/skycot/RUNBOOK.md)
  — full canonical runbook.
- [Operations → Failure modes](./failure-modes.md) — what the
  platform protects against and what it doesn't.
