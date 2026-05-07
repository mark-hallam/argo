# Argo / Skycot

Argo is an agent platform for running role-bound, evidence-aware,
approval-gated AI workers. The commercial worker layer is called
**Skycot**.

This book is the unified reference for both audiences:

- **Operators** running Skycot workers (sales SDR, customer support,
  marketing intern, marketplace seller, …) — start with the
  [Quickstart](./quickstart/install.md).
- **Developers** extending the core platform or authoring vertical
  templates — start with [Protocol overview](./architecture/protocol.md)
  and the [Five non-negotiable invariants](./reference/invariants.md).

## What problem Argo solves

A single engineer can fully audit Argo on a weekend. A 200-person
company can deploy it to production without writing the multi-tenant
layer themselves.

The platform is built around a five-invariant kernel
(see [the invariants](./reference/invariants.md)) and a worker
runtime that observes every action through an append-only audit
chain. Approvals, spend, memory, and skill learning are all
first-class concerns of the platform — not features bolted on by
each vertical template.

## Where to next

| You want to … | Start here |
|---|---|
| Install Argo and run your first worker | [Quickstart → Install](./quickstart/install.md) |
| Pick a vertical (sales / support / …) | [How-to → Pick a vertical](./howto/pick-a-vertical.md) |
| Wire up Slack / SMS / email approvals | [How-to → Set up approval channels](./howto/approvals.md) |
| Look up a CLI command | [Reference → CLI](./reference/cli.md) |
| Understand the protocol | [Architecture → Protocol overview](./architecture/protocol.md) |
| Resolve an incident | [Operations → Operator runbook](./operations/runbook.md) |

## How this book is organised

The book mirrors the [Diátaxis](https://diataxis.fr/) structure:

- **Quickstart** — task-oriented tutorials that take you from zero
  to a running worker.
- **How-to** — task-oriented recipes for specific operator goals.
- **Reference** — exhaustive catalog of CLI commands, configuration,
  and platform invariants.
- **Architecture** — explanation of the design and the why behind
  the protocol.
- **Operations** — what to do when something breaks.

Most pages link out to the canonical source documents in
[`docs/skycot/`](https://github.com/mark-hallam/Argo-Agent/tree/main/docs/skycot)
and [`docs/build-plan/`](https://github.com/mark-hallam/Argo-Agent/tree/main/docs/build-plan)
rather than duplicating their content. The goal of the book is a
single discoverable surface — the canonical content stays close to
the code.
