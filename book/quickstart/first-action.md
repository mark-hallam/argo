# Your first action

You have a worker that has emitted at least one action in the
`pending` state. This page walks you through approving and
executing it.

## Inspect the action

```bash
argo worker actions show <action-id> --json
```

The output includes the action's kind (e.g. `mcp`,
`computer_use`, `code_execution`, `document_creation`,
`filesystem`), the executor, the inputs the worker is asking for
permission to use, and a confidence score.

## Approve

```bash
argo worker actions approve <action-id>
```

Approval flows through the workspace's auto-approval policy:

- If the policy auto-approves the action, it is moved to
  `approved` and queued for execution.
- If the policy escalates (high spend, sensitive scope, low
  confidence), Argo dispatches a notify event to every configured
  channel — push, email, Slack, SMS, voice, iMessage, WhatsApp,
  webinbox. The action stays in `pending` until a human responds.

## Execute

```bash
argo worker actions execute <action-id>
```

The executor named on the action runs the inputs through the
matching `ExecutorKind` handler (CU, MCP, code, doc, filesystem).
Results are stored in the action's bundle directory under
`<workspace>/.skycot/outputs/<job>/actions/<id>/`.

## Confirm completion

```bash
argo worker actions list --state completed --vertical gtm-engineer --json
```

The `completed` action carries an `executed_at` timestamp and the
final state score. The associated audit chain has rows for the
state transitions; verify chain integrity with:

```bash
argo audit query --session <session-id> --decision approved
```

## What just happened

You exercised the full v1 demo gate:

1. Genesis (or pre-loaded vertical)
2. Worker turn
3. Action emitted
4. Auto-approval gate
5. Channel notify (or auto-approve)
6. Executor dispatch
7. Audit chain row

If any of those steps doesn't complete, walk
[`./scripts/smoke-skycot.sh`](https://github.com/mark-hallam/Argo-Agent/blob/main/scripts/smoke-skycot.sh)
which exercises every transition in mock-mode against a tempdir
workspace — the smoke test is a useful diff baseline.

## Next

- [How-to → Set up approval channels](../howto/approvals.md)
- [Reference → CLI](../reference/cli.md) for every command
- [Operations → Operator runbook](../operations/runbook.md) when
  something breaks
