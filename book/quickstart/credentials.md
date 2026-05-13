# Configure credentials

Argo can run the first worker with the mock provider, but a real
operator run needs an LLM provider key and at least one approval
channel.

## Add an LLM key

Use Anthropic for the first run:

```bash
export ANTHROPIC_API_KEY="sk-ant-replace-me"
argo cred set ANTHROPIC_API_KEY "$ANTHROPIC_API_KEY"
argo cred list
```

Expected output:

```text
ANTHROPIC_API_KEY=********
```

If `argo cred list` is empty, check that you ran the command from the
workspace folder created in the previous step.

## Create the worker

```bash
argo worker create \
  --vertical marketing-intern \
  --persona Maya \
  --charter "Plan one weekly campaign, draft the first post, and ask before publishing." \
  --builder you@example.com \
  --json
```

Expected output:

```json
{
  "id": "019...",
  "tenant_id": "019...",
  "vertical_id": "marketing-intern",
  "persona": {
    "name": "Maya"
  }
}
```

Copy the `id` and `tenant_id` values. The rest of this guide calls
them `<worker-id>` and `<tenant-id>`.

## Add an approval channel

Add Slack as the first approval channel:

```bash
argo worker channels add <worker-id> slack:C0123456789 --tenant <tenant-id> --json
```

Expected output:

```json
{
  "ok": true,
  "worker_id": "019...",
  "spec": "slack:C0123456789",
  "added": true,
  "duplicate": false,
  "total": 1
}
```

If you do not have a Slack channel id yet, use an email target instead:

```bash
argo worker channels add <worker-id> email:you@example.com --tenant <tenant-id> --json
```

Expected output:

```json
{
  "ok": true,
  "worker_id": "019...",
  "spec": "email:you@example.com",
  "added": true,
  "duplicate": false,
  "total": 1
}
```

If the command reports an unknown worker, check the copied worker id
with:

```bash
argo worker list --tenant <tenant-id>
```

## Next

Run Maya once in [Your first worker](./first-worker.md).
