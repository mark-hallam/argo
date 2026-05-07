# Pick a vertical

Argo ships several demo verticals out of the box. Each is a
template — the actual worker is a Genesis-generated instance bound
to your tenant.

| Vertical | Use case |
|---|---|
| `gtm-engineer` | Marketplace seller — research and list a product |
| `customer-support` | Inbox triage + draft responses |
| `sales-sdr` | Cold outreach + follow-up cadence |
| `marketing-intern` | Content drafts + research summaries |

## List verticals on your install

```bash
argo vertical list
```

## Inspect a vertical

```bash
argo vertical show <id>
argo vertical describe <id>
```

`describe` prints the manifest, role, persona, memory schema,
declared executors, and approval policy in a human-readable form.

## Create a new vertical with Genesis

If none of the demo verticals match your role, use Genesis:

```bash
argo genesis interactive
```

You'll be guided through six prompts (role / name / industry /
duties / channels / spend cap). The output is a vertical template
written to `<workspace>/.skycot/verticals/<id>/` plus an audit row
recording the charter.

For headless creation:

```bash
argo genesis plan --charter charter.yaml
argo genesis refine <id>
```

## Pull from the marketplace

Verticals can be published as OCI artifacts and pulled by other
operators:

```bash
argo vertical publish <id> --registry <oci-ref>
argo vertical pull <oci-ref>
```

The marketplace flow is documented in
[`docs/skycot/RUNBOOK.md`](https://github.com/mark-hallam/Argo-Agent/blob/main/docs/skycot/RUNBOOK.md)
under the "Marketplace" heading.

## Next

After picking a vertical, set up [approval channels](./approvals.md)
so escalations can reach a human.
