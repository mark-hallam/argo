# Five non-negotiable invariants

The platform is defined by five invariants. Anything that
violates them is a bug.

## 1. Polyglot core

The Rust core exposes a JSON-RPC plugin ABI. Plugins (executors,
connectors, channels) run as subprocesses or out-of-process
servers. The core never `dlopen`s an arbitrary plugin in-process.

This is what makes the platform **a single engineer can fully
audit on a weekend**. The core is small; everything else lives at
the JSON-RPC seam.

## 2. Multi-tenant by construction

`tenant_id` is plumbed through every primitive — memory rows,
audit rows, sessions, workers, channels, skills. Cross-tenant
queries are explicit and gated. The store enforces isolation by
indexed predicate.

This is what makes the platform deployable **without writing the
multi-tenant layer yourself**.

## 3. Capability-based security with first-class RBAC

Every dispatch checks declared capabilities. A vertical's
manifest declares which executor kinds it can use; a worker's
roster declares which capabilities are available; a tenant's
policy can tighten further (caps tighten only).

Signatures escalate to the maximum across the lineage. Quiet
hours always apply at the lowest level.

## 4. Provider manifests, not adapters

Provider quirks (Anthropic vs OpenAI vs Mock) live in TOML
manifests, not Rust code. The runtime reads the manifest and
configures a provider client. Adding a new provider is a TOML
change plus a thin client-side compatibility shim — never a fork
of the core.

## 5. MCP-first integration

Eight in-core tools (memory, skill, audit, judge, plan, etc.).
Everything else is an MCP server.

| Connector | MCP server crate |
|---|---|
| Gmail | `argo-mcp-connector-gmail` |
| Slack | `argo-mcp-connector-slack` |
| Google Calendar | `argo-mcp-connector-gcal` |
| HubSpot | `argo-mcp-connector-hubspot` |
| Notion | `argo-mcp-connector-notion` |
| Stripe | `argo-mcp-connector-stripe` |
| Asana | `argo-mcp-connector-asana` |
| Salesforce | `argo-mcp-connector-salesforce` |
| Linear | `argo-mcp-connector-linear` |
| ClickUp | `argo-mcp-connector-clickup` |
| Monday.com | `argo-mcp-connector-monday` |
| PagerDuty | `argo-mcp-connector-pagerduty` |
| Fireflies | `argo-mcp-connector-fireflies` |
| Jira | `argo-mcp-connector-jira` |
| Airtable | `argo-mcp-connector-airtable` |

(Approval channels follow the same pattern — Slack, Twilio SMS,
Twilio voice, Twilio WhatsApp, Postmark email, Expo push, Blue
Bubbles iMessage, embedded webinbox.)

## Hard rules CI enforces

These flow from the invariants:

- No file > 500 LOC.
- No fn > 80 LOC.
- No struct impl block > 200 LOC.
- No `unsafe` Rust outside marked FFI shims.
- No `unwrap` / `expect` outside `#[cfg(test)]`.
- No global mutable state. `AgentDeps` is the only injection point.
- Every public Rust API has rustdoc with at least one example.

CI rejects any PR that violates these.

## When in doubt

- **Subtract before adding.** Idiot Index = solution complexity ÷
  problem complexity.
- **Question the requirement.** If something doesn't make sense,
  raise it; do not invent.
- **Ship the 80% solution.** Working software beats the perfect
  solution that arrives late.
