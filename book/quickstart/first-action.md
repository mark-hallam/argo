# Your first action

The worker has proposed an action and is waiting for approval. This
step shows the approval gate, execution, and audit trail.

## Inspect the action

```bash
argo worker actions show <action-id> --json
```

Expected output:

```json
{
  "action_id": "019...",
  "state": "pending",
  "requires_approval": true,
  "action_type": "draft_post",
  "risk_level": "medium"
}
```

If you do not have an action id, run:

```bash
argo worker actions list --state pending --vertical marketing-intern
```

## Approve it

```bash
argo worker actions approve <action-id> --approved-by you@example.com --json
```

Expected output:

```json
{
  "action_id": "019...",
  "state": "approved",
  "approved_by": "you@example.com"
}
```

If the command says the action is not pending, inspect the latest
state with `argo worker actions show <action-id> --json`.

## Execute it

```bash
argo worker actions execute <action-id> --json
```

Expected output:

```json
{
  "action_id": "019...",
  "state": "executing",
  "execution_bundle_path": ".skycot/outputs/019.../actions/019..."
}
```

Most executors write evidence under `.skycot/outputs/<job-id>/actions/`.
For browser or Slack actions, the evidence includes the request summary
and the response metadata. Some executors complete immediately; others
stay in `executing` until the operator records the result.

Complete the action when the result is ready:

```bash
argo worker actions complete <action-id> --note "Published draft reviewed." --json
```

Expected output:

```json
{
  "action_id": "019...",
  "state": "completed",
  "result": {
    "summary": "Published draft reviewed."
  }
}
```

If execution fails because a connector is missing, configure that
connector and run the execute command again. The action ledger is
append-only, so the failure remains visible for audit.

## Confirm completion

```bash
argo worker actions list --state completed --vertical marketing-intern --json
```

Expected output:

```json
{"action_id":"019...","state":"completed","executed_at":"2026-05-13T...Z"}
```

Check the audit chain:

```bash
argo audit query --session <session-id> --decision approved
```

Expected output:

```text
approved  action_id=019...  operator=you@example.com
```

If you do not know the session id yet, use:

```bash
argo work logs <job-id> --kind transcript
```

The transcript includes the worker turn that produced the action.

## What happened

You exercised the operator loop:

1. You delegated work to Maya.
2. Maya planned and proposed an action.
3. Argo held the action for approval.
4. You approved it.
5. The executor ran it.
6. The audit chain recorded the transition.

## Next

Go to [Next steps](./next-steps.md).
