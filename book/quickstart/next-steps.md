# Next steps

You now have a worker, a queued job, an approval decision, execution
evidence, and audit history.

## Keep operating

Run the worker continuously:

```bash
argo work serve \
  --vertical marketing-intern \
  --provider anthropic \
  --model claude-sonnet-4-6
```

Expected output:

```text
argo work: queue empty, sleeping 5s
```

Add more work from another terminal:

```bash
argo work add \
  --vertical marketing-intern \
  --prompt "Summarise this week's campaign ideas and ask before sending anything."
```

Expected output:

```text
queued job 019... for vertical marketing-intern
```

## Read next

- [How-to: pick a vertical](../howto/pick-a-vertical.md) for choosing a better starting template.
- [How-to: set up approval channels](../howto/approvals.md) for Slack and email approval routing.
- [How-to: troubleshoot a stuck worker](../howto/troubleshoot.md) for common first-run errors.
- [Reference: CLI](../reference/cli.md) for every command.
- [Operations: operator runbook](../operations/runbook.md) for production setup and incidents.

## Hand off to a teammate

Show the worker state:

```bash
argo worker show <worker-id> --tenant <tenant-id>
```

Expected output:

```text
worker_id      019...
tenant_id      019...
vertical_id    marketing-intern
persona        Maya
```

Send the worker id, tenant id, and the approval-channel setup page to
the person who will approve Maya's actions.
