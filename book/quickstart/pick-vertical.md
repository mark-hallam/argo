# Pick a vertical

A vertical is the reusable job template for a worker. Pick one that
matches the job you want to delegate.

```bash
argo vertical list
```

Expected output includes the shipped demo verticals:

```text
marketplace-seller
customer-support
sales-sdr
marketing-intern
finance-ar
```

Use this menu:

| Vertical | Use this if |
| --- | --- |
| `marketplace-seller` | You want a worker to research, draft, and post marketplace listings after approval. |
| `customer-support` | You want inbox triage, suggested replies, and escalation for support queues. |
| `sales-sdr` | You want lead research, outbound drafts, and follow-up cadence support. |
| `marketing-intern` | You want campaign research, content drafts, and light reporting. |
| `finance-ar` | You want invoice review, reminder drafts, and payment follow-up. |

For this quickstart, use `marketing-intern`:

```bash
argo vertical show marketing-intern
```

Expected output:

```text
id: marketing-intern
name: Marketing Intern
actions: ...
```

If `marketing-intern` is missing, run `argo vertical list --source both`
from the workspace root. If it is still missing, install or pull the
starter library before continuing.

## Next

Connect the provider and approval channel in
[Configure credentials](./credentials.md).
