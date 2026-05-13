# Your first worker

Maya is now registered as a `marketing-intern` worker. Give her one
small job and run a single worker pass.

## Queue work

```bash
argo work add \
  --vertical marketing-intern \
  --prompt "Draft a launch post for a local bookkeeping service. Ask before publishing." \
  --json
```

Expected output:

```json
{
  "ok": true,
  "job_id": "019...",
  "vertical_id": "marketing-intern",
  "status": "pending"
}
```

Copy the `job_id`. You can always find it again with:

```bash
argo work jobs --state pending --vertical marketing-intern
```

Expected output:

```text
job_id                                state      vertical
019...                                pending    marketing-intern
```

If no pending job appears, check that you used the same vertical id in
both commands.

## Run one pass

```bash
argo work serve \
  --vertical marketing-intern \
  --provider anthropic \
  --model claude-sonnet-4-6 \
  --once \
  --json
```

Expected output:

```json
{
  "ok": true,
  "job_id": "019...",
  "session_id": "019...",
  "completed_at": "2026-05-13T...Z",
  "stop_reason": "approval_required",
  "actions_emitted": 1
}
```

If you see `provider credentials not configured`, return to
[Configure credentials](./credentials.md) and set
`ANTHROPIC_API_KEY`. If you want a dry local check first, use
`--provider mock --model mock`; it produces deterministic output
without contacting a provider.

## Confirm the worker produced output

```bash
argo work jobs --state completed --vertical marketing-intern
```

Expected output:

```text
job_id                                state      vertical
019...                                completed  marketing-intern
```

Then inspect the worker's proposed actions:

```bash
argo worker actions list --state pending --vertical marketing-intern --json
```

Expected output:

```json
{"action_id":"019...","state":"pending","vertical_id":"marketing-intern","requires_approval":true}
```

If no pending action appears, the worker may have completed the task
without an approval-gated action. Re-run with a prompt that explicitly
asks Maya to publish or send something, then inspect the list again.

## Next

Approve and execute an action in [Your first action](./first-action.md).
