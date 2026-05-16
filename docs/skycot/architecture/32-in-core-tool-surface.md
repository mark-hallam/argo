# 32 — In-core tool surface + MCP connectors

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Update when:** tool added/removed from `crates/argo-tool/src/tools/`, or MCP connector added/removed.

SKYCOT exposes two tool surfaces: **in-core tools** (compiled into the gateway binary, ~15 tools per `crates/argo-tool/src/tools/mod.rs`) and **MCP connectors** (37 subprocess-spawned tools per `crates/argo-mcp-connector-*`). The architecture invariant: **MCP-first integration** — only 8 tools were specified for in-core; the rest live as MCP servers.

## Diagram

```mermaid
flowchart TB
    WORKER[Worker / agent runner<br/>argo-agent-runner]
    
    REGISTRY[ToolRegistry<br/>argo-tool::ToolRegistry]
    
    subgraph INCORE[In-core tools §9 + Hermes-parity]
        T1[file_read<br/>workspace-scoped]
        T2[file_write<br/>workspace-scoped]
        T3[file_patch<br/>diff apply]
        T4[terminal<br/>+ streaming]
        T5[process<br/>spawn helper]
        T6[memory<br/>workspace/team/agent]
        T7[delegate_task<br/>sub-worker spawn]
        T8[send_message<br/>multi-channel ask + tell]
        T9[web_search<br/>via MCP server stub]
        T10[web_extract<br/>via MCP server stub]
        T11[clarify<br/>operator round-trip]
        T12[todo<br/>worker scratchpad]
        T13[code_execute<br/>python/node/bash<br/>via argo-sandbox]
        T14[vision<br/>provider multimodal]
        T15[session_search<br/>retro lookup]
    end
    
    subgraph MCP[MCP connectors 37 — subprocess]
        subgraph PROD[Productivity]
            M_GMAIL[gmail]
            M_GCAL[gcal]
            M_GMEET[google-meet]
            M_NOTION[notion]
            M_AIRTABLE[airtable]
            M_ONEDRIVE[onedrive]
        end
        subgraph CRM[CRM/Sales]
            M_HUBSPOT[hubspot]
            M_SALESFORCE[salesforce]
            M_APOLLO[apollo]
            M_ZOOMINFO[zoominfo]
        end
        subgraph PM[Project mgmt]
            M_LINEAR[linear]
            M_JIRA[jira]
            M_ASANA[asana]
            M_MONDAY[monday]
            M_TRELLO[trello]
            M_CLICKUP[clickup]
            M_GH[github-issues]
        end
        subgraph COMM[Channels backed]
            M_SLACK[slack]
            M_DISCORD[discord]
            M_IMSG[imessage]
            M_SIGNAL[signal]
            M_TG[telegram]
            M_ZOOM[zoom]
        end
        subgraph EMAIL_TX[Email transactional]
            M_SENDGRID[sendgrid]
            M_POSTMARK[postmark]
            M_MAILGUN[mailgun]
            M_RESEND[resend]
            M_MAILCHIMP[mailchimp]
        end
        subgraph BILLING[Billing/Payments]
            M_STRIPE[stripe]
            M_STRIPE_EV[stripe-events]
        end
        subgraph SUPPORT[Support/Ops]
            M_ZENDESK[zendesk]
            M_PAGERDUTY[pagerduty]
            M_TYPEFORM[typeform]
            M_CALENDLY[calendly]
            M_FIREFLIES[fireflies]
            M_FIGMA[figma]
        end
    end
    
    POOL[McpClientPool<br/>argo-mcp<br/>keyed by tenant_id]
    SUBPROC[Subprocess per tenant + connector<br/>stdio JSON-RPC]
    
    WORKER --> REGISTRY
    REGISTRY --> INCORE
    REGISTRY --> POOL
    POOL --> SUBPROC
    SUBPROC -.-> PROD
    SUBPROC -.-> CRM
    SUBPROC -.-> PM
    SUBPROC -.-> COMM
    SUBPROC -.-> EMAIL_TX
    SUBPROC -.-> BILLING
    SUBPROC -.-> SUPPORT

    classDef worker fill:#fef3c7,stroke:#d97706
    classDef reg fill:#dbeafe,stroke:#2563eb
    classDef incore fill:#dcfce7,stroke:#16a34a
    classDef mcp fill:#f3e8ff,stroke:#9333ea
    classDef pool fill:#fed7aa,stroke:#ea580c

    class WORKER worker
    class REGISTRY reg
    class INCORE incore
    class MCP mcp
    class POOL,SUBPROC pool
```

## In-core tool roster (canonical)

| Tool | Module | Status | Capability gate |
|---|---|---|---|
| `file_read` | `tools/file_read.rs` | End-to-end | `Capability::FsRead{path_glob}` |
| `file_write` | `tools/file_write.rs` | End-to-end | `Capability::FsWrite{path_glob}` |
| `file_patch` | `tools/file_patch.rs` | End-to-end | `Capability::FsWrite{path_glob}` |
| `terminal` | `tools/terminal.rs` + `terminal_streaming.rs` | End-to-end | `Capability::ToolDispatch{terminal}` |
| `process` | `tools/process.rs` | End-to-end | `Capability::Spawn` |
| `memory` | `tools/memory.rs` + `_namespace_tests.rs` | End-to-end (F-03 #1737) | `Capability::MemoryWrite` |
| `delegate_task` | `tools/delegate_task.rs` | End-to-end | `Capability::Spawn` |
| `send_message` | `tools/send_message.rs` | End-to-end (F-07 #1729) | `Capability::MessageSend{channel, scope}` |
| `web_search` | `tools/web/search.rs` | MCP-backed (schema-only stub) | `Capability::NetEgress` |
| `web_extract` | `tools/web/extract.rs` | MCP-backed (schema-only stub) | `Capability::NetEgress` |
| `clarify` | `tools/clarify.rs` | End-to-end | (no gate — escalation primitive) |
| `todo` | `tools/todo.rs` | End-to-end | (worker-local) |
| `code_execute` | `tools/code_execute.rs` | End-to-end (sandboxed) | `Capability::LocalCodeExecution` (F-12 in-flight) |
| `vision` | `tools/vision.rs` | End-to-end | `Capability::ToolDispatch{vision}` |
| `session_search` | `tools/session_search.rs` | End-to-end | `Capability::ToolDispatch{session_search}` |

## MCP connector roster (37 total)

| Category | Connectors | Status notes |
|---|---|---|
| Productivity | gmail, gcal, google-meet, notion, airtable, onedrive | Production-ready |
| CRM/Sales | hubspot, salesforce, apollo, zoominfo | apollo + zoominfo: DEFERRED-v1.x (enterprise stubs) |
| PM | linear, jira, asana, monday, trello, clickup, github-issues | Production-ready |
| Comms | slack, discord, imessage, signal, telegram, zoom | signal + telegram: SCHEDULED-DELETE-v1 (consumer); zoom: DEFERRED-v1.x |
| Email tx | sendgrid, postmark, mailgun, resend, mailchimp | mailgun + resend: SCHEDULED-DELETE-v1 (redundant) |
| Billing | stripe, stripe-events | Production-ready |
| Support/Ops | zendesk, pagerduty, typeform, calendly, fireflies, figma | Production-ready |
| Shared scaffold | `argo-mcp-connector-shared` | Common subprocess plumbing |

## Tool dispatch flow

```mermaid
sequenceDiagram
    participant W as Worker turn
    participant R as ToolRegistry
    participant CAP as CapabilityEngine
    participant T as Tool impl
    participant MCP as McpClientPool
    participant SUB as MCP subprocess
    participant A as AuditSink

    W->>R: dispatch(name, args, ctx)
    R->>CAP: tool.required_capabilities(args)
    CAP->>CAP: match against role caps
    
    alt allow
        R->>T: tool.run(ctx, args)
        alt In-core
            T-->>R: ToolResult
        else MCP-backed
            T->>MCP: get_client(tenant_id, connector)
            MCP->>SUB: spawn or reuse subprocess
            SUB-->>MCP: JSON-RPC response
            MCP-->>T: ToolResult
            T-->>R: ToolResult
        end
        R->>A: audit ToolDispatch + ToolResult
        R-->>W: result
    else deny
        R->>A: audit ToolDispatch (denied)
        R-->>W: ❌ Capability
    end
```

## Key invariants

- **MCP-first** — if it's not one of the 8 §9 + 7 Hermes-parity tools, it's MCP. No new tools land in-core without architectural review.
- **MCP pool keyed by tenant.** Different tenants never share subprocess credentials. (Per F-04 #1727 multi-tenancy.)
- **Workspace-scoped fs by default.** `file_read`/`file_write` paths interpolated against `{workspace_root}` (F-06 #1723).
- **Per-channel send capability.** `send_message` requires `Capability::MessageSend{channel, recipient_scope}`; `ToolDispatch{send_message}` alone is no longer enough (F-07 #1729).
- **No `unwrap`/`expect` outside test code** — CI gates this.

## Source-of-truth pointers

- Tool trait: `crates/argo-tool/src/tool.rs::Tool`
- Tool registry: `crates/argo-tool/src/registry.rs::ToolRegistry`
- Tools mod: `crates/argo-tool/src/tools/mod.rs`
- MCP pool: `crates/argo-mcp/src/pool.rs::McpClientPool`
- Send-message dispatcher: `crates/argo-tool/src/tools/send_message.rs::MessageDispatcher`
- Connector crates: `crates/argo-mcp-connector-*` (37 total)
- Connector shared lib: `crates/argo-mcp-connector-shared/`
