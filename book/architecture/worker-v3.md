# Worker v3 execution

Worker v3 is the generic execution layer that turns LLM-emitted
actions into platform-mediated effects. Every action has a finite
state machine; every transition writes an audit row.

The full design spec is
[`docs/design/worker_v3_execution.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/design/worker_v3_execution.md).

## Action state machine

```
pending ──approve──▶ approved ──execute──▶ executing ──complete──▶ completed
   │                    │                       │
   │                    │                       └──fail──▶ failed
   └──cancel──▶ cancelled
```

Each transition is gated:

- `pending → approved`: gated by the auto-approval policy.
- `approved → executing`: gated by spend budget + tenant quota.
- `executing → completed | failed`: terminal. Determined by the
  executor.
- `pending | approved → cancelled`: explicit operator action.

## Executor kinds

Argo ships several `ExecutorKind` variants. Each has a dedicated
crate and handler:

| ExecutorKind | Crate | Use for |
|---|---|---|
| `computer_use` | `argo-executor-computer-use` | Browser / desktop CU via Anthropic |
| `mcp` | wired through `argo-mcp-connector-*` | Tool calls into a connector MCP server |
| `code_execution` | (sandbox stub) | Python / Node code via E2B |
| `document_creation` | `argo-executor-document` | Markdown / CSV / PDF / slide decks |
| `filesystem` | `argo-executor-filesystem` | Sandboxed FS ops (read/write/list/…) |
| `api` | (planned) | Direct REST calls per manifest |

A vertical's manifest declares which executor kinds it can use.

## Auto-approval policy

`AutoApprovalPolicy` is the gate between `pending` and `approved`.
Policy resolves through a lineage:

```
worker ← roster (multi-roster collapses via most_restrictive) ← workspace
```

The resolved policy gates:

- spend budget (daily / monthly)
- wake-action types (which executor kinds are allowed)
- signature requirements (one or more channel sigs)
- quiet hours (always apply at the lowest level)

See [Reference → Configuration](../reference/configuration.md) for
the policy schema.

## Notify on Decision::Ask

When the policy returns `Decision::Ask`, Argo dispatches a notify
event to every channel in `<workspace>/.skycot/notify.toml`. The
action stays `pending` until a human responds via any channel's
decision callback.

## Audit chain

Every transition writes a row to the audit chain. Rows are
hash-linked; replay verifies the full chain. The chain is queried
through `argo audit query` (filters: decision / worker / session /
escalation) and replayed through `argo audit replay <session>`.

## Real-time stream

The audit chain has a broadcast channel. Operators tail it via:

```bash
argo activity tail
GET /audit/stream  # SSE-equivalent NDJSON
```

## Where to read more

- [`docs/skycot/RUNBOOK.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/skycot/RUNBOOK.md)
  — operator-level walkthrough of approve / execute / cancel.
- [`docs/skycot/operator_runbook.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/skycot/operator_runbook.md)
  — `argo worker actions` CLI reference + ledger artifact map.
- [`docs/design/worker_v3_execution.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/design/worker_v3_execution.md)
  — design spec.
